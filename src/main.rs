use better_default::Default;
use core::ops::{Deref, DerefMut};
use tracing::error;

use bytes::BytesMut;
use iced::widget::{column, *};
use iced::{Element, widget::image::Handle};
use iced::{
    Length::Fill,
    widget::canvas::{Cache, Canvas, Image},
};

fn main() -> iced::Result {
    iced::run(update, view)
}

fn update(state: &mut State, message: Message) {}

fn view(state: &State) -> Element<'_, Message> {
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
            buffer: buffer,
            cache: Cache::default(),
        }
    }
}

impl<Message> canvas::Program<Message> for Buffer {
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

#[derive(Default, Debug)]
struct Registers {
    sp: u16,
    pc: u16,
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: FlagsRegister,
    h: u8,
    l: u8,
}

impl Registers {
    fn af(&self) -> u16 {
        (self.a as u16) << 8 | *self.f as u16
    }
    fn set_af(&mut self, value: u16) {
        self.a = ((value & 0xFF00) >> 8) as u8;
        *self.f = (value & 0xFF) as u8;
    }

    fn bc(&self) -> u16 {
        (self.b as u16) << 8 | self.c as u16
    }
    fn set_bc(&mut self, value: u16) {
        self.b = ((value & 0xFF00) >> 8) as u8;
        self.c = (value & 0xFF) as u8;
    }
    fn de(&self) -> u16 {
        (self.d as u16) << 8 | self.e as u16
    }
    fn set_de(&mut self, value: u16) {
        self.d = ((value & 0xFF00) >> 8) as u8;
        self.e = (value & 0xFF) as u8;
    }

    fn hl(&self) -> u16 {
        (self.h as u16) << 8 | self.l as u16
    }
    fn set_hl(&mut self, value: u16) {
        self.h = ((value & 0xFF00) >> 8) as u8;
        self.l = (value & 0xFF) as u8;
    }
}

#[repr(transparent)]
#[derive(Default, Debug, Copy, Clone)]
struct FlagsRegister(u8);
impl FlagsRegister {
    fn new(value: u8) -> Self {
        Self(value)
    }

    fn zero(&self) -> bool {
        (self.0 >> 7) & 1 != 0
    }
    fn subtract(&self) -> bool {
        (self.0 >> 6) & 1 != 0
    }
    fn half_carry(&self) -> bool {
        (self.0 >> 5) & 1 != 0
    }
    fn carry(&self) -> bool {
        (self.0 >> 4) & 1 != 0
    }

    fn set_zero(&mut self, value: bool) {
        self.0 |= u8::from(value) << 7;
    }
    fn set_subtract(&mut self, value: bool) {
        self.0 |= u8::from(value) << 6;
    }
    fn set_half_carry(&mut self, value: bool) {
        self.0 |= u8::from(value) << 5;
    }
    fn set_carry(&mut self, value: bool) {
        self.0 |= u8::from(value) << 4;
    }
}

impl<'a> From<u8> for FlagsRegister {
    fn from(value: u8) -> Self {
        Self::new(value)
    }
}

impl<'a> DerefMut for FlagsRegister {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> Deref for FlagsRegister {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
enum Operation {
    Add(Target, Target),
    Nop,
    Halt,
    Load(Target, Target),
    Inc(Target),
    Dec(Target),
    Stop,
    JumpRelative(i8),
    RotateAccumulator(RotationType, Direction),
    ComplementCarry,
    SetCarry,
    ComplementAccumulator,
    DecimalAdjustAccumulator,
}

#[derive(Debug, Clone, Copy)]
enum Registers8 {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
}
#[derive(Debug, Clone, Copy)]
enum Registers16 {
    BC,
    DE,
    HL,
    SP,
}

enum Indirect {
    R16(Registers16),
    Imm16(u16),
    HLI,
    HLD,
    C(u8),
}

enum Target {
    R8(Registers8),
    R16(Registers16),
    Imm8(u8),
    Imm16(u16),
    Ind(Indirect),
}

enum Condition {
    None,
    NZ,
    Z,
    NC,
    C,
}

enum RotationType {
    Circular,
    NonCircular,
}

enum Direction {
    Left,
    Right,
}

struct Instruction {
    operation: Operation,
}

type Memory16K = [u8; 1024 * 16];
type Memory8K = [u8; 1024 * 8];
type Memory4K = [u8; 1024 * 4];

#[derive(Default)]
struct MemoryBus {
    #[default([0; 1024*16])]
    rom: Memory16K,
    #[default(vec![[0; 1024*16]])]
    rom_banks: Vec<Memory16K>,
    #[default([0; 1024 * 8])]
    vram: Memory8K,
    #[default([0; 1024 * 8])]
    external_ram: Memory8K,
    #[default([0; 1024 * 4])]
    wram1: Memory4K,
    #[default([0; 1024 * 4])]
    wram2: Memory4K,
    #[default([0; 0xFFFE-0xFF80])]
    hram: [u8; 0xFFFE - 0xFF80],
    ie: u8,
}

impl MemoryBus {
    fn read_u8(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => self.rom[address as usize - 0x0000],
            0x4000..=0x7FFF => {
                // TODO: switchable rom banks
                self.rom_banks[0][address as usize - 0x4000]
            }
            0x8000..=0x9FFF => self.vram[address as usize - 0x8000],
            0xA000..=0xBFFF => self.external_ram[address as usize - 0xA000],
            0xC000..=0xCFFF => self.wram1[address as usize - 0xC000],
            0xD000..=0xDFFF => self.wram2[address as usize - 0xD000],
            0xE000..=0xFDFF => {
                //Echo RAM
                self.wram1[address as usize - 0xE000]
            }
            0xFE00..=0xFE9F => {
                todo!("Implement OAM")
            }
            0xFEA0..=0xFEFF => {
                todo!("Prohibited region, implement undefined behaviour")
            }
            0xFF00..=0xFF7F => {
                todo!("IO registers")
            }
            0xFF80..=0xFFFE => self.hram[address as usize - 0xFF80],
            0xFFFF => self.ie,
        }
    }

    fn set_u8(&mut self, address: u16, value: u8) {
        *match address {
            0x0000..=0x3FFF => &mut self.rom[address as usize - 0x0000],
            0x4000..=0x7FFF => {
                // TODO: switchable rom banks
                &mut self.rom_banks[0][address as usize - 0x4000]
            }
            0x8000..=0x9FFF => &mut self.vram[address as usize - 0x8000],
            0xA000..=0xBFFF => &mut self.external_ram[address as usize - 0xA000],
            0xC000..=0xCFFF => &mut self.wram1[address as usize - 0xC000],
            0xD000..=0xDFFF => &mut self.wram2[address as usize - 0xD000],
            0xE000..=0xFDFF => {
                //Echo RAM
                &mut self.wram1[address as usize - 0xE000]
            }
            0xFE00..=0xFE9F => {
                todo!("Implement OAM")
            }
            0xFEA0..=0xFEFF => {
                todo!("Prohibited region, implement undefined behaviour")
            }
            0xFF00..=0xFF7F => {
                todo!("IO registers")
            }
            0xFF80..=0xFFFE => &mut self.hram[address as usize - 0xFF80],
            0xFFFF => &mut self.ie,
        } = value;
    }
}

#[derive(Default)]
struct CPU {
    registers: Registers,
    pc: u16,
    memory: MemoryBus,
    ir: u8,
}

impl CPU {
    fn tick(&mut self) {
        self.pc = self.pc.wrapping_add(1);
    }

    fn cycle(&mut self) {
        use Registers8::*;
        use Registers16::*;
        use Target::*;
        let operation = match decompose_octal(self.ir) {
            // https://gbdev.io/gb-opcodes/optables/octal
            (0o0, 0o0, 0o0) => Operation::Nop,
            (0o0, 0o1, 0o0) => {
                self.tick();
                let lsb = self.memory.read_u8(self.pc);
                self.tick();
                let msb = self.memory.read_u8(self.pc);
                Operation::Load(
                    Ind(Indirect::Imm16(u16::from_le_bytes([lsb, msb]))),
                    R16(SP),
                )
            }
            (0o0, 0o2, 0o0) => Operation::Stop,
            (0o0, condition, 0o0) => {
                use Condition::*;
                let condition = match condition {
                    0o3 => None,
                    0o4 => NZ,
                    0o5 => Z,
                    0o6 => NC,
                    0o7 => C,
                    _ => unreachable!(),
                };
                self.tick();
                let offset = self.memory.read_u8(self.pc) as i8;
                Operation::JumpRelative(offset)
            }
            (0o0, op, 0o1) => {
                let target = match op >> 1 {
                    0 => BC,
                    1 => DE,
                    2 => HL,
                    3 => SP,
                    _ => unreachable!(),
                };
                if op & 0b1 == 0 {
                    self.tick();
                    let lsb = self.memory.read_u8(self.pc);
                    self.tick();
                    let msb = self.memory.read_u8(self.pc);
                    Operation::Load(R16(target), Imm16(u16::from_le_bytes([lsb, msb])))
                } else {
                    Operation::Add(R16(HL), R16(target))
                }
            }
            (0o0, op, 0o2) => {
                let other = match op >> 1 {
                    0 => Indirect::R16(BC),
                    1 => Indirect::R16(DE),
                    2 => Indirect::HLI,
                    3 => Indirect::HLD,
                    _ => unreachable!(),
                };
                if op & 0b1 == 0 {
                    Operation::Load(Ind(other), R8(A))
                } else {
                    Operation::Load(R8(A), Ind(other))
                }
            }
            (0o0, op, 0o3) => {
                let target = match op >> 1 {
                    0 => BC,
                    1 => DE,
                    2 => HL,
                    3 => SP,
                    _ => unreachable!(),
                };
                if op & 0b1 == 0 {
                    Operation::Inc(R16(target))
                } else {
                    Operation::Dec(R16(target))
                }
            }
            (0o0, target, 0o4) => {
                let target = match target {
                    0o0 => R8(B),
                    0o1 => R8(C),
                    0o2 => R8(D),
                    0o3 => R8(E),
                    0o4 => R8(H),
                    0o5 => R8(L),
                    0o6 => Ind(Indirect::R16(HL)),
                    0o7 => R8(A),
                    _ => unreachable!(),
                };
                Operation::Inc(target)
            }
            (0o0, target, 0o5) => {
                let target = match target {
                    0o0 => R8(B),
                    0o1 => R8(C),
                    0o2 => R8(D),
                    0o3 => R8(E),
                    0o4 => R8(H),
                    0o5 => R8(L),
                    0o6 => Ind(Indirect::R16(HL)),
                    0o7 => R8(A),
                    _ => unreachable!(),
                };
                Operation::Dec(target)
            }
            (0o0, destination, 0o6) => {
                let destination = match destination {
                    0o0 => R8(B),
                    0o1 => R8(C),
                    0o2 => R8(D),
                    0o3 => R8(E),
                    0o4 => R8(H),
                    0o5 => R8(L),
                    0o6 => Ind(Indirect::R16(HL)),
                    0o7 => R8(A),
                    _ => unreachable!(),
                };
                self.tick();
                let value = self.memory.read_u8(self.pc);
                Operation::Load(destination, Imm8(value))
            }
            (0o0, op @ 0o0..=0o3, 0o7) => {
                let kind = match op >> 1 {
                    0o0 => RotationType::Circular,
                    0o1 => RotationType::NonCircular,
                    _ => unreachable!(),
                };
                let direction = if op & 0b1 == 0 {
                    Direction::Left
                } else {
                    Direction::Right
                };
                Operation::RotateAccumulator(kind, direction)
            }
            (0o0, 0o4, 0o7) => Operation::DecimalAdjustAccumulator,
            (0o0, 0o5, 0o7) => Operation::ComplementAccumulator,
            (0o0, 0o6, 0o7) => Operation::SetCarry,
            (0o0, 0o7, 0o7) => Operation::ComplementCarry,
            (0o1, 0o5, 0o6) => Operation::Halt,
            (0o1, destination @ 0o0..=0o7, source @ _) => {
                let destination = match destination {
                    0o0 => R8(B),
                    0o1 => R8(C),
                    0o2 => R8(D),
                    0o3 => R8(E),
                    0o4 => R8(H),
                    0o5 => R8(L),
                    0o6 => Ind(Indirect::R16(HL)),
                    0o7 => R8(A),
                    _ => unreachable!(),
                };
                let source = match source {
                    0o0 => R8(B),
                    0o1 => R8(C),
                    0o2 => R8(D),
                    0o3 => R8(E),
                    0o4 => R8(H),
                    0o5 => R8(L),
                    0o6 => Ind(Indirect::R16(HL)),
                    0o7 => R8(A),
                    _ => unreachable!(),
                };

                Operation::Load(destination, source)
            }
            (0o2..=0o3, _, _) => {
                todo!()
            }
            (0o4.., _, _) | (_, 0o10.., _) | (_, _, 0o10..) => unreachable!(),
        };
    }
}

fn decompose_octal(value: u8) -> (u8, u8, u8) {
    ((value >> 6) & 0o7, (value >> 3) & 0o7, value & 0o7)
}
