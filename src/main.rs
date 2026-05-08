#![allow(clippy::upper_case_acronyms)]
#![feature(uint_gather_scatter_bits)]
use better_default::Default;

use bytes::BytesMut;
use iced::widget::canvas::{Cache, Image};
use iced::widget::{column, *};
use iced::{Element, widget::image::Handle};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, fmt};

use crate::context::Context;

pub(crate) mod clock;
pub(crate) mod context;
pub(crate) mod cpu;
pub(crate) mod ppu;

fn main() -> iced::Result {
    let format = fmt::format()
        .with_level(false) // don't include levels in formatted output
        .with_target(false) // don't include targets
        .without_time()
        .compact();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(LevelFilter::DEBUG.into()))
        .event_format(format)
        .init();

    let mut ctx = Context::default();
    let mut cpu = cpu::CPU::default();
    // cpu.load_debug_initial_state();
    // cpu.load_rom(include_bytes!(
    //     "../test_roms/cpu_instrs/individual/08-misc instrs.gb"
    // ));

    cpu.load_boot_rom(include_bytes!("../bootrom/dmg_boot.bin"), &mut ctx);
    cpu.run(&mut ctx);
    //iced::run(update, view)
    Ok(())
}

fn update(_state: &mut State, _message: Message) {}

fn view(_state: &State) -> Element<'_, Message> {
    column![
        text("Test"),
        canvas(Buffer::default()).width(160 * 3).height(144 * 3)
    ]
    .into()
}

#[derive(Debug, Clone, Default)]
struct State {}

#[derive(Debug, Clone)]
enum Message {}

struct Buffer {
    buffer: BytesMut,
    cache: Cache,
}
impl Default for Buffer {
    fn default() -> Self {
        let mut buffer = BytesMut::zeroed(160 * 144 * 4);
        for pixel in buffer.as_chunks_mut::<4>().0 {
            pixel[3] = 0xFF
        }

        Self {
            buffer,
            cache: Cache::default(),
        }
    }
}

impl<Message> canvas::Program<Message> for Buffer {
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

            frame.draw_image(bounds, image);
        });

        vec![screen]
    }

    fn update(
        &self,
        _state: &mut Self::State,
        _event: &iced::Event,
        _bounds: iced::Rectangle,
        _cursor: iced::advanced::mouse::Cursor,
    ) -> Option<Action<Message>> {
        None
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: iced::Rectangle,
        _cursor: iced::advanced::mouse::Cursor,
    ) -> iced::advanced::mouse::Interaction {
        iced::advanced::mouse::Interaction::default()
    }
}
