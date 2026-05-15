#![allow(clippy::upper_case_acronyms)]
#![feature(uint_gather_scatter_bits)]
use core::time::Duration;
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    rc::Rc,
};

use better_default::Default;

use bytes::{Bytes, BytesMut};
use iced::{Element, Padding, Rectangle, Task, exit, widget::image::Handle, window::Settings};
use iced::{
    Subscription,
    widget::canvas::{Cache, Image},
};
use iced::{
    widget::{column, *},
    window,
};
use tap::Pipe;
use tracing::{debug, info, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, fmt};

use gbemu::{
    context::{Context, Memory, MemoryBus},
    cpu,
    ppu::{self, Mode},
};

fn main() -> iced::Result {
    let format = fmt::format()
        .with_level(false) // don't include levels in formatted output
        .with_target(false) // don't include targets
        .without_time()
        .compact();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .event_format(format)
        .init();

    dioxus_devtools::connect_subsecond();

    subsecond::call(|| {
        iced::daemon(App::new, App::update, App::view)
            .subscription(App::subscription)
            .title(App::title)
            .run()
    })?;

    Ok(())
}

#[derive(Debug)]
struct Window {
    window_type: WindowType,
    title: String,
}
impl Window {
    fn new(window_type: WindowType) -> Self {
        Self {
            window_type,
            title: match window_type {
                WindowType::Main => "gbemu".into(),
                WindowType::TileViewer => "Tile Viewer".into(),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum WindowType {
    Main,
    TileViewer,
}

#[derive(Default)]
struct App {
    windows: BTreeMap<window::Id, Window>,
    gameboy: GameBoy,
    tile_viewer: TileViewer,
    is_playing: bool,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let (_, main_window) = window::open(Settings::default());
        let (_, tile_viewer) = window::open(Settings::default());

        (
            Self::default(),
            Task::batch([
                main_window.map(|id| Message::WindowOpened(id, WindowType::Main)),
                tile_viewer.map(|id| Message::WindowOpened(id, WindowType::TileViewer)),
            ]),
        )
    }
    fn view(&self, window_id: window::Id) -> Element<'_, Message> {
        subsecond::call(|| {
            if let Some(window) = self.windows.get(&window_id) {
                match window.window_type {
                    WindowType::Main => {
                        column![
                            row![
                                button("Start").on_press(GameBoyMessage::Play.into()),
                                button("Toggle Playback")
                                    .on_press(GameBoyMessage::TogglePlayback.into()),
                                button("Tick").on_press(GameBoyMessage::ManualTick.into())
                            ],
                            canvas(&self.gameboy).width(160 * 3).height(144 * 3)
                        ]
                    }
                    WindowType::TileViewer => {
                        column![
                            text("Tile Viewer"),
                            canvas(&self.tile_viewer)
                                .width(8 * 16 * 3)
                                .height(8 * 24 * 3)
                        ]
                    }
                }
            } else {
                column![]
            }
            .into()
        })
    }

    fn title(&self, window_id: window::Id) -> String {
        if let Some(window) = self.windows.get(&window_id) {
            window.title.clone()
        } else {
            "gbemu".into()
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        subsecond::call(|| {
            match message {
                Message::GameBoyMessage(message) => match message {
                    tick_type @ (GameBoyMessage::ManualTick | GameBoyMessage::Tick) => {
                        self.gameboy
                            .tick(matches!(tick_type, GameBoyMessage::ManualTick));
                        if (self.gameboy.ppu.current_mode == Mode::VBlank
                            && self.gameboy.context.memory.io.lcd.ly == 144
                            && self.gameboy.ppu.cycle_counter == 0)
                            || matches!(tick_type, GameBoyMessage::ManualTick)
                        {
                            self.tile_viewer.tiles = self
                                .gameboy
                                .context
                                .memory
                                .vram
                                .tile_data()
                                .pipe(|data| data.as_chunks::<16>().0)
                                .iter()
                                .map(|x| {
                                    x.pipe(|data| data.as_chunks::<2>().0)
                                        .iter()
                                        .cloned()
                                        .flat_map(|[left, right]| {
                                            let row = ((left as u16) << 8) | right as u16;
                                            (0..8)
                                                .map(move |index| {
                                                    row.extract_bits(0b1000_0000_1000_0000 >> index)
                                                })
                                                .flat_map(|colour| match colour {
                                                    0 => [0xFF, 0xFF, 0xFF, 0xFF],
                                                    1 => [0xBC, 0xBC, 0xBC, 0xFF],
                                                    2 => [0x80, 0x80, 0x80, 0xFF],
                                                    _ => [0x0, 0x0, 0x0, 0xFF],
                                                })
                                        })
                                        .collect()
                                })
                                .collect();
                            self.tile_viewer.cache.clear();
                        }
                    }
                    GameBoyMessage::Play => {
                        let Some(rom_path) = rfd::FileDialog::new()
                            .add_filter("GameBoy ROMs", &["gb"])
                            .set_directory(env::current_dir().unwrap_or_else(|_| {
                                env::home_dir().unwrap_or_else(|| PathBuf::from("/"))
                            }))
                            .pick_file()
                        else {
                            return Task::none();
                        };
                        self.gameboy
                            .cpu
                            .load_debug_initial_state(&mut self.gameboy.context);
                        // state.gameboy.cpu.load_boot_rom(
                        //     include_bytes!("../bootrom/dmg_boot.bin"),
                        //     &mut state.gameboy.context,
                        // );

                        let rom = fs::read(rom_path).unwrap();
                        self.gameboy.cpu.load_rom(&rom, &mut self.gameboy.context);
                        self.is_playing = true;
                    }
                    GameBoyMessage::TogglePlayback => {
                        self.is_playing = !self.is_playing;
                    }
                },
                Message::WindowOpened(id, window_type) => {
                    let window = Window::new(window_type);
                    self.windows.insert(id, window);
                }
                Message::WindowClosed(id) => {
                    if let Some(Window {
                        window_type: WindowType::Main,
                        ..
                    }) = self.windows.get(&id)
                    {
                        return exit();
                    }
                }
            }
            Task::none()
        })
    }
    fn subscription(&self) -> Subscription<Message> {
        let timer = if self.is_playing {
            iced::time::every(Duration::from_nanos(238)).map(|_| GameBoyMessage::Tick.into())
        } else {
            Subscription::none()
        };

        let window_events = window::events().filter_map(|(id, event)| match event {
            window::Event::Closed => Some(Message::WindowClosed(id)),
            _ => None,
        });

        Subscription::batch([timer, window_events])
    }
}

#[derive(Clone, Debug, Copy)]

enum Message {
    WindowOpened(window::Id, WindowType),
    GameBoyMessage(GameBoyMessage),
    WindowClosed(window::Id),
}
#[derive(Clone, Debug, Copy)]

enum GameBoyMessage {
    ManualTick,
    Tick,
    Play,
    TogglePlayback,
}
impl From<GameBoyMessage> for Message {
    fn from(value: GameBoyMessage) -> Self {
        Message::GameBoyMessage(value)
    }
}

#[derive(Debug, Default)]
struct TileViewer {
    tiles: Vec<Bytes>,
    cache: Cache,
}
impl<Message> canvas::Program<Message> for TileViewer {
    type State = ();

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &iced_renderer::core::Theme,
        bounds: iced::Rectangle,
        cursor: iced::advanced::mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let screen = self.cache.draw(renderer, bounds.size(), |frame| {
            for y in 0..24 {
                for x in 0..16 {
                    let image = Image::from(&Handle::from_rgba(
                        8,
                        8,
                        self.tiles.get(y * 16 + x).map_or_else(
                            || (0..64).flat_map(|_| [0, 0, 0, 255]).collect(),
                            |x| x.clone(),
                        ),
                    ))
                    .snap(true)
                    .filter_method(image::FilterMethod::Nearest);
                    frame.draw_image(
                        Rectangle::new(
                            iced::Point {
                                x: x as f32 * 8.0 * 3.0,
                                y: y as f32 * 8.0 * 3.0,
                            },
                            iced::Size::from([8.0 * 3.0; 2]),
                        ),
                        image,
                    );
                }
            }
        });

        vec![screen]
    }
}

struct GameBoy {
    buffer: BytesMut,
    cache: Cache,
    context: Context<MemoryBus>,
    cpu: cpu::CPU<MemoryBus>,
    ppu: ppu::PPU,
    counter: u64,
}
impl GameBoy {
    fn tick(&mut self, manual: bool) -> bool {
        if self.counter.is_multiple_of(4) {
            self.cpu.tick(&mut self.context);
        }
        self.ppu.tick(&mut self.context);

        self.counter = self.counter.wrapping_add(1);

        if (self.ppu.current_mode == Mode::VBlank
            && self.context.memory.io.lcd.ly == 144
            && self.ppu.cycle_counter == 0)
            || manual
        {
            self.buffer = self
                .ppu
                .screen
                .iter()
                .flat_map(|pixel| match pixel {
                    ppu::Pixel::White => [220, 220, 220, 255],
                    ppu::Pixel::LightGray => [160, 160, 160, 255],
                    ppu::Pixel::DarkGrey => [80, 80, 80, 255],
                    ppu::Pixel::Black => [0, 0, 0, 255],
                })
                .collect();

            self.cache.clear();
        }
        false
    }
}

impl Default for GameBoy {
    fn default() -> Self {
        let context = Context::default();
        let cpu = cpu::CPU::default();
        let ppu = ppu::PPU::default();

        let mut buffer = BytesMut::zeroed(160 * 144 * 4);
        for pixel in buffer.as_chunks_mut::<4>().0 {
            pixel[3] = 0xFF
        }

        Self {
            buffer,
            cache: Cache::default(),
            context,
            cpu,
            ppu,
            counter: 0,
        }
    }
}

impl<Message> canvas::Program<Message> for GameBoy {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &iced_renderer::core::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::advanced::mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let screen = self.cache.draw(renderer, bounds.size(), |frame| {
            let image = Image::from(&Handle::from_rgba(160, 144, self.buffer.clone()))
                .snap(true)
                .filter_method(image::FilterMethod::Nearest);

            frame.draw_image(bounds.shrink(Padding::from([77, 80])), image);
        });

        vec![screen]
    }
}
