use better_default::Default;
use core::ops::{Deref, DerefMut, Shl, Shr};
use tracing::error;

use bytes::BytesMut;
use iced::widget::{column, *};
use iced::{Element, widget::image::Handle};
use iced::{
    Length::Fill,
    widget::canvas::{Cache, Canvas, Image},
};
use tap::{Conv, Pipe, Tap};
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

    fn set_zero(&mut self, value: bool) -> &mut Self {
        self.0 |= u8::from(value) << 7;
        self
    }
    fn set_subtract(&mut self, value: bool) -> &mut Self {
        self.0 |= u8::from(value) << 6;
        self
    }
    fn set_half_carry(&mut self, value: bool) -> &mut Self {
        self.0 |= u8::from(value) << 5;
        self
    }
    fn set_carry(&mut self, value: bool) -> &mut Self {
        self.0 |= u8::from(value) << 4;
        self
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
    Nop,
    Halt,
    Load(Target, Target),
    Inc(Target),
    Dec(Target),
    Stop,
    JumpRelative(Condition, i8),
    RotateAccumulator(RotationType, Direction),
    ComplementCarry,
    SetCarry,
    ComplementAccumulator,
    DecimalAdjustAccumulator,
    Compare(Target),
    Or(Target),
    Xor(Target),
    And(Target),
    Sbc(Target),
    Sub(Target),
    Adc(Target),
    Add(Target, Target),
    Return(Condition),
    Push(Registers16),
    Pop(Registers16),
    ReturnInterrupt,
    Jump(Condition, Target),
    AddStack(i8),
    LoadStackOffset(i8),
    DisableInterrupt,
    EnableInterrupt,
    Call(Condition, Target),
    Restart(u16),
    Rotate(RotationType, Direction, Target),
    ShiftArithmetic(Direction, Target),
    Swap(Target),
    ShiftRightLogical(Target),
    TestBit(u8, Target),
    ResetBit(u8, Target),
    SetBit(u8, Target),
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
    AF,
}
#[derive(Debug, Clone, Copy)]

enum Indirect {
    R16(Registers16),
    Imm16(u16),
    HLI,
    HLD,
    C,
}

enum Target {
    R8(Registers8),
    R16(Registers16),
    Imm8(u8),
    Imm16(u16),
    Ind(Indirect),
}
#[derive(Debug, Clone, Copy)]

enum Condition {
    None,
    NZ,
    Z,
    NC,
    C,
}
#[derive(Debug, Clone, Copy)]

enum RotationType {
    Circular,
    NonCircular,
}
#[derive(Debug, Clone, Copy)]

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
    ime: bool,
}

impl CPU {
    fn tick(&mut self) {}
    fn tick_and_increment_pc(&mut self) {
        self.tick();
        self.pc = self.pc.wrapping_add(1);
    }

    fn fetch(&mut self) {
        use Registers8::*;
        use Registers16::*;
        use Target::*;
        let operation = match decompose_octal_triplet(self.ir) {
            // https://gbdev.io/gb-opcodes/optables/octal
            (0o0, 0o0, 0o0) => Operation::Nop,
            (0o0, 0o1, 0o0) => {
                let lsb = self.memory.read_u8(self.pc);
                self.tick_and_increment_pc();
                let msb = self.memory.read_u8(self.pc);
                self.tick_and_increment_pc();
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

                let offset = self.memory.read_u8(self.pc) as i8;
                self.tick_and_increment_pc();
                Operation::JumpRelative(condition, offset)
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
                    let lsb = self.memory.read_u8(self.pc);
                    self.tick_and_increment_pc();
                    let msb = self.memory.read_u8(self.pc);
                    self.tick_and_increment_pc();
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
                let value = self.memory.read_u8(self.pc);
                self.tick_and_increment_pc();
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
            (0o2, op, target) => {
                let target = match target {
                    0 => R8(B),
                    1 => R8(C),
                    2 => R8(D),
                    3 => R8(E),
                    4 => R8(H),
                    5 => R8(L),
                    6 => Ind(Indirect::R16(HL)),
                    7 => R8(A),
                    _ => unreachable!(),
                };
                match op {
                    0 => Operation::Add(R8(A), target),
                    1 => Operation::Adc(target),
                    2 => Operation::Sub(target),
                    3 => Operation::Sbc(target),
                    4 => Operation::And(target),
                    5 => Operation::Xor(target),
                    6 => Operation::Or(target),
                    7 => Operation::Compare(target),
                    _ => unreachable!(),
                }
            }
            (0o3, condition @ 0o0..=0o3, 0o0) => {
                use Condition::*;
                let condition = match condition {
                    0o0 => NZ,
                    0o1 => Z,
                    0o2 => NC,
                    0o3 => C,
                    _ => unreachable!(),
                };
                Operation::Return(condition)
            }
            (0o3, kind @ (0o4 | 0o6), 0o0) => {
                let offset = self.memory.read_u8(self.pc);
                self.tick_and_increment_pc();
                let address = u16::from_le_bytes([offset, 0xFF]);
                match kind {
                    0o4 => Operation::Load(Ind(Indirect::Imm16(address)), R8(A)),
                    0o6 => Operation::Load(R8(A), Ind(Indirect::Imm16(address))),
                    _ => unreachable!(),
                }
            }
            (0o3, 0o5, 0o0) => {
                let offset = self.memory.read_u8(self.pc) as i8;
                self.tick_and_increment_pc();
                Operation::AddStack(offset)
            }
            (0o3, 0o7, 0o0) => {
                let offset = self.memory.read_u8(self.pc) as i8;
                self.tick_and_increment_pc();
                Operation::LoadStackOffset(offset)
            }
            (0o3, 0o7, 0o1) => Operation::Load(R16(SP), R16(HL)),
            (0o3, target @ (0 | 2 | 4 | 6), kind @ (0o1 | 0o5)) => {
                let target = match target {
                    0 => BC,
                    2 => DE,
                    4 => HL,
                    6 => AF,
                    _ => unreachable!(),
                };
                match kind {
                    0o1 => Operation::Pop(target),
                    0o5 => Operation::Push(target),
                    _ => unreachable!(),
                }
            }
            (0o3, 0o1, 0o1) => Operation::Return(Condition::None),
            (0o3, 0o3, 0o1) => Operation::ReturnInterrupt,
            (0o3, 0o5, 0o1) => Operation::Jump(Condition::None, R16(HL)),
            (0o3, condition @ 0o0..=0o3, 0o2) => {
                use Condition::*;
                let condition = match condition {
                    0o0 => NZ,
                    0o1 => Z,
                    0o2 => NC,
                    0o3 => C,
                    _ => unreachable!(),
                };
                let lsb = self.memory.read_u8(self.pc);
                self.tick_and_increment_pc();
                let msb = self.memory.read_u8(self.pc);
                self.tick_and_increment_pc();
                let address = u16::from_le_bytes([lsb, msb]);
                Operation::Jump(condition, Imm16(address))
            }
            (0o3, op @ 0o4..=0o7, 0o2) => {
                let dest = if op & 1 == 0 {
                    Ind(Indirect::C)
                } else {
                    let lsb = self.memory.read_u8(self.pc);
                    self.tick_and_increment_pc();
                    let msb = self.memory.read_u8(self.pc);
                    self.tick_and_increment_pc();
                    let address = u16::from_le_bytes([lsb, msb]);
                    Ind(Indirect::Imm16(address))
                };
                let source = R8(A);
                match op {
                    0o4..=0o5 => Operation::Load(source, dest),
                    0o6..=0o7 => Operation::Load(dest, source),
                    _ => unreachable!(),
                }
            }
            (0o3, 0o0, 0o3) => {
                let lsb = self.memory.read_u8(self.pc);
                self.tick_and_increment_pc();
                let msb = self.memory.read_u8(self.pc);
                self.tick_and_increment_pc();
                let address = u16::from_le_bytes([lsb, msb]);
                Operation::Jump(Condition::None, Imm16(address))
            }
            (0o3, 0o1, 0o3) => self.fetch_cb_operation(),
            (0o3, 0o2..=0o5, 0o3) => {
                error!("Invalid opcode");
                todo!("Decide what to do on invalid opcode")
            }
            (0o3, 0o6, 0o3) => Operation::DisableInterrupt,
            (0o3, 0o7, 0o3) => Operation::EnableInterrupt,
            (0o3, condition @ 0o0..=0o3, 0o4) => {
                use Condition::*;
                let condition = match condition {
                    0o0 => NZ,
                    0o1 => Z,
                    0o2 => NC,
                    0o3 => C,
                    _ => unreachable!(),
                };
                let lsb = self.memory.read_u8(self.pc);
                self.tick_and_increment_pc();
                let msb = self.memory.read_u8(self.pc);
                self.tick_and_increment_pc();
                let address = u16::from_le_bytes([lsb, msb]);
                Operation::Call(condition, Imm16(address))
            }
            (0o3, 0o4..=0o7, 0o4) => {
                error!("Invalid opcode");
                todo!("Decide what to do on invalid opcode")
            }
            (0o3, 0o1, 0o5) => {
                let lsb = self.memory.read_u8(self.pc);
                self.tick_and_increment_pc();
                let msb = self.memory.read_u8(self.pc);
                self.tick_and_increment_pc();
                let address = u16::from_le_bytes([lsb, msb]);
                Operation::Call(Condition::None, Imm16(address))
            }
            (0o3, 0o3 | 0o5 | 0o7, 0o5) => {
                error!("Invalid opcode");
                todo!("Decide what to do on invalid opcode")
            }
            (0o3, kind @ 0o0..=0o7, 0o6) => {
                let value = self.memory.read_u8(self.pc);
                self.tick_and_increment_pc();
                let dest = R8(A);
                let operation = match kind {
                    0o0 => |value| Operation::Add(R8(A), value),
                    0o1 => Operation::Adc,
                    0o2 => Operation::Sub,
                    0o3 => Operation::Sbc,
                    0o4 => Operation::And,
                    0o5 => Operation::Xor,
                    0o6 => Operation::Or,
                    0o7 => Operation::Compare,
                    _ => unreachable!(),
                };
                operation(Target::Imm8(value))
            }
            (0o3, variant @ 0o0..=0o7, 0o7) => {
                let lsb = match variant {
                    0o0 => 0x00,
                    0o1 => 0x08,
                    0o2 => 0x10,
                    0o3 => 0x18,
                    0o4 => 0x20,
                    0o5 => 0x28,
                    0o6 => 0x30,
                    0o7 => 0x38,
                    _ => unreachable!(),
                };
                let address = u16::from_le_bytes([lsb, 0x00]);
                Operation::Restart(address)
            }
            (0o4.., _, _) | (_, 0o10.., _) | (_, _, 0o10..) => unreachable!(),
        };
    }

    fn fetch_cb_operation(&mut self) -> Operation {
        use Registers8::*;
        use Registers16::*;
        use Target::*;
        let (operation, target) = decompose_octal_cb(self.memory.read_u8(self.pc));
        self.tick_and_increment_pc();

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

        let operation = match operation {
            0o0 => Operation::Rotate(RotationType::Circular, Direction::Left, target),
            0o1 => Operation::Rotate(RotationType::Circular, Direction::Right, target),
            0o2 => Operation::Rotate(RotationType::NonCircular, Direction::Left, target),
            0o3 => Operation::Rotate(RotationType::NonCircular, Direction::Right, target),
            0o4 => Operation::ShiftArithmetic(Direction::Left, target),
            0o5 => Operation::ShiftArithmetic(Direction::Right, target),
            0o6 => Operation::Swap(target),
            0o7 => Operation::ShiftRightLogical(target),
            number @ 0o10..=0o17 => Operation::TestBit(number - 0o10, target),
            number @ 0o20..=0o27 => Operation::ResetBit(number - 0o20, target),
            number @ 0o30..=0o37 => Operation::SetBit(number - 0o30, target),
            0o40.. => unreachable!(),
        };
        operation
    }

    fn execute_operation(&mut self, operation: Operation) {
        match operation {
            Operation::Nop => {}
            Operation::Halt => self.halt(),
            Operation::Load(destination, source) => self.load(destination, source),
            Operation::Inc(target) => self.increment(target),
            Operation::Dec(target) => self.decrement(target),
            Operation::Stop => self.stop(),
            Operation::Jump(condition, target) => self.jump(condition, target),
            Operation::JumpRelative(condition, offset) => self.jump_relative(condition, offset),
            Operation::RotateAccumulator(rotation_type, direction) => {
                self.rotate_accumulator(rotation_type, direction)
            }
            Operation::ComplementCarry => {
                let new_carry = !self.registers.f.carry();
                self.registers
                    .f
                    .set_subtract(false)
                    .set_half_carry(false)
                    .set_carry(new_carry);
            }
            Operation::SetCarry => {
                self.registers
                    .f
                    .set_subtract(false)
                    .set_half_carry(false)
                    .set_carry(true);
            }
            Operation::ComplementAccumulator => {
                self.registers.a = !self.registers.a;
                self.registers.f.set_subtract(true).set_half_carry(true);
            }
            Operation::DecimalAdjustAccumulator => {
                let mut adjustment = 0u8;
                if self.registers.f.subtract() {
                    if self.registers.f.half_carry() {
                        adjustment = adjustment.wrapping_add(0x6);
                    }
                    if self.registers.f.carry() {
                        adjustment = adjustment.wrapping_add(0x60);
                    }
                    self.registers.a = self.registers.a.wrapping_sub(adjustment);
                } else {
                    if self.registers.f.half_carry() || self.registers.a & 0xF > 0x9 {
                        adjustment = adjustment.wrapping_add(0x6);
                    }
                    if self.registers.f.carry() || self.registers.a > 0x99 {
                        adjustment = adjustment.wrapping_add(0x60);
                        self.registers.f.set_carry(true);
                    }
                    self.registers.a = self.registers.a.wrapping_add(adjustment);
                }
                self.registers
                    .f
                    .set_zero(self.registers.a == 0)
                    .set_half_carry(false);
            }
            Operation::Compare(target) => {
                let value = match target {
                    Target::R8(register) => self.read_register8(register),
                    Target::Imm8(value) => value,
                    Target::Ind(Indirect::R16(register)) => {
                        let value = self.memory.read_u8(self.read_register16(register));
                        self.tick();
                        value
                    }
                    _ => unimplemented!("Invalid targets for operation"),
                };
                let (result, carry) = self.registers.a.overflowing_sub(value);
                self.registers
                    .f
                    .set_zero(result == 0)
                    .set_subtract(true)
                    .set_half_carry(
                        (self.registers.a & 0xF).wrapping_sub(value & 0xF) & 0x10 == 0x10,
                    )
                    .set_carry(carry);
            }
            Operation::Or(target) => {
                let value = match target {
                    Target::R8(register) => self.read_register8(register),
                    Target::Imm8(value) => value,
                    Target::Ind(Indirect::R16(register)) => {
                        let value = self.memory.read_u8(self.read_register16(register));
                        self.tick();
                        value
                    }
                    _ => unimplemented!("Invalid targets for operation"),
                };
                let result = self.registers.a | value;
                self.registers
                    .f
                    .set_zero(result == 0)
                    .set_subtract(false)
                    .set_half_carry(false)
                    .set_carry(false);
                self.registers.a = result;
            }
            Operation::Xor(target) => {
                let value = match target {
                    Target::R8(register) => self.read_register8(register),
                    Target::Imm8(value) => value,
                    Target::Ind(Indirect::R16(register)) => {
                        let value = self.memory.read_u8(self.read_register16(register));
                        self.tick();
                        value
                    }
                    _ => unimplemented!("Invalid targets for operation"),
                };
                let result = self.registers.a ^ value;
                self.registers
                    .f
                    .set_zero(result == 0)
                    .set_subtract(false)
                    .set_half_carry(false)
                    .set_carry(false);
                self.registers.a = result;
            }
            Operation::And(target) => {
                let value = match target {
                    Target::R8(register) => self.read_register8(register),
                    Target::Imm8(value) => value,
                    Target::Ind(Indirect::R16(register)) => {
                        let value = self.memory.read_u8(self.read_register16(register));
                        self.tick();
                        value
                    }
                    _ => unimplemented!("Invalid targets for operation"),
                };
                let result = self.registers.a & value;
                self.registers
                    .f
                    .set_zero(result == 0)
                    .set_subtract(false)
                    .set_half_carry(false)
                    .set_carry(false);
                self.registers.a = result;
            }
            Operation::Sbc(target) => {
                let value = match target {
                    Target::R8(register) => self.read_register8(register),
                    Target::Imm8(value) => value,
                    Target::Ind(Indirect::R16(register)) => {
                        let value = self.memory.read_u8(self.read_register16(register));
                        self.tick();
                        value
                    }
                    _ => unimplemented!("Invalid targets for operation"),
                };
                let incoming_carry = self.registers.f.carry() as u8;
                let (result, carry1) = self.registers.a.overflowing_sub(value);
                let (result, carry2) = result.overflowing_sub(incoming_carry);

                self.registers
                    .f
                    .set_zero(result == 0)
                    .set_subtract(true)
                    .set_half_carry(
                        (self.registers.a & 0xF)
                            .wrapping_sub(value & 0xF)
                            .wrapping_sub(incoming_carry & 0xF)
                            & 0x10
                            == 0x10,
                    )
                    .set_carry(carry1 || carry2);
                self.registers.a = result;
            }
            Operation::Sub(target) => {
                let value = match target {
                    Target::R8(register) => self.read_register8(register),
                    Target::Imm8(value) => value,
                    Target::Ind(Indirect::R16(register)) => {
                        let value = self.memory.read_u8(self.read_register16(register));
                        self.tick();
                        value
                    }
                    _ => unimplemented!("Invalid targets for operation"),
                };
                let (result, carry) = self.registers.a.overflowing_sub(value);
                self.registers
                    .f
                    .set_zero(result == 0)
                    .set_subtract(true)
                    .set_half_carry(
                        (self.registers.a & 0xF).wrapping_sub(value & 0xF) & 0x10 == 0x10,
                    )
                    .set_carry(carry);
                self.registers.a = result;
            }
            Operation::Adc(target) => {
                let value = match target {
                    Target::R8(register) => self.read_register8(register),
                    Target::Imm8(value) => value,
                    Target::Ind(Indirect::R16(register)) => {
                        let value = self.memory.read_u8(self.read_register16(register));
                        self.tick();
                        value
                    }
                    _ => unimplemented!("Invalid targets for operation"),
                };
                let incoming_carry = self.registers.f.carry() as u8;
                let (result, carry1) = self.registers.a.overflowing_add(value);
                let (result, carry2) = result.overflowing_add(incoming_carry);

                self.registers
                    .f
                    .set_zero(result == 0)
                    .set_subtract(false)
                    .set_half_carry(
                        (self.registers.a & 0xF)
                            .wrapping_add(value & 0xF)
                            .wrapping_add(incoming_carry & 0xF)
                            & 0x10
                            == 0x10,
                    )
                    .set_carry(carry1 || carry2);
                self.registers.a = result;
            }
            Operation::Add(destination, source) => match destination {
                Target::R8(Registers8::A) => {
                    let value = match source {
                        Target::R8(register) => self.read_register8(register),
                        Target::Imm8(value) => value,
                        Target::Ind(Indirect::R16(register)) => {
                            let value = self.memory.read_u8(self.read_register16(register));
                            self.tick();
                            value
                        }
                        _ => unimplemented!("Invalid targets for operation"),
                    };
                    let incoming_carry = self.registers.f.carry() as u8;
                    let (result, carry1) = self.registers.a.overflowing_sub(value);
                    let (result, carry2) = result.overflowing_sub(incoming_carry);

                    self.registers
                        .f
                        .set_zero(result == 0)
                        .set_subtract(true)
                        .set_half_carry(
                            (self.registers.a & 0xF)
                                .wrapping_sub(value & 0xF)
                                .wrapping_sub(incoming_carry & 0xF)
                                & 0x10
                                == 0x10,
                        )
                        .set_carry(carry1 || carry2);
                    self.registers.a = result;
                }
                Target::R16(Registers16::HL) => match source {
                    Target::R16(register) => {
                        let (l, r) = (self.registers.hl(), self.read_register16(register));
                        let (result, carry) = l.overflowing_add(r);
                        let quarter_carry = ((l & 0xF) + (r & 0xF)) & 0x10 == 0x10;
                        let half_carry = ((l & 0xFF) + (r & 0xFF)) & 0x100 == 0x100;
                        let three_quarter_carry = ((l & 0xFFF) + (r & 0xFFF)) & 0x1000 == 0x1000;
                        let [lsb, msb] = result.to_le_bytes();
                        self.registers.l = lsb;
                        self.registers
                            .f
                            .set_subtract(false)
                            .set_half_carry(quarter_carry)
                            .set_carry(half_carry);
                        self.tick();
                        self.registers.h = msb;
                        self.registers
                            .f
                            .set_subtract(false)
                            .set_half_carry(three_quarter_carry)
                            .set_carry(carry);
                    }
                    _ => unimplemented!("Invalid operation"),
                },
                _ => unimplemented!("Invalid operation"),
            },
            Operation::Return(condition) => {
                let condition_met = match condition {
                    Condition::None => true,
                    Condition::NZ => !self.registers.f.zero(),
                    Condition::Z => self.registers.f.zero(),
                    Condition::NC => !self.registers.f.carry(),
                    Condition::C => self.registers.f.carry(),
                };
                if condition_met {
                    if !matches!(condition, Condition::None) {
                        self.tick()
                    };
                    let lsb = self.memory.read_u8(self.registers.sp);
                    self.registers.sp = self.registers.sp.wrapping_add(1);
                    self.tick();
                    let msb = self.memory.read_u8(self.registers.sp);
                    self.registers.sp = self.registers.sp.wrapping_add(1);
                    self.tick();
                    self.pc = u16::from_le_bytes([lsb, msb]);
                }
                self.tick();
            }
            Operation::Push(register) => {
                let [lsb, msb] = self.read_register16(register).to_le_bytes();
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.tick();
                self.memory.set_u8(self.registers.sp, msb);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.tick();
                self.memory.set_u8(self.registers.sp, lsb);
                self.tick();
            }
            Operation::Pop(register) => {
                let lsb = self.memory.read_u8(self.registers.sp);
                self.registers.sp = self.registers.sp.wrapping_add(1);
                self.tick();
                let msb = self.memory.read_u8(self.registers.sp);
                self.registers.sp = self.registers.sp.wrapping_add(1);
                self.tick();
                self.set_register16(register, u16::from_le_bytes([lsb, msb]));
            }
            Operation::ReturnInterrupt => {
                self.tick();
                let lsb = self.memory.read_u8(self.registers.sp);
                self.registers.sp = self.registers.sp.wrapping_add(1);
                self.tick();
                let msb = self.memory.read_u8(self.registers.sp);
                self.registers.sp = self.registers.sp.wrapping_add(1);
                self.tick();
                self.pc = u16::from_le_bytes([lsb, msb]);
                self.ime = true;
                self.tick();
            }
            Operation::AddStack(offset) => {
                let [lsb, msb] = self.registers.sp.to_le_bytes();
                let (result_lsb, carry) = lsb.overflowing_add(offset as u8);
                self.registers
                    .f
                    .set_zero(false)
                    .set_subtract(false)
                    .set_half_carry((lsb & 0xF).wrapping_add(offset as u8 & 0xF) & 0x10 == 0x10)
                    .set_carry(carry);
                let z_sign = (result_lsb >> 7) & 0b1 == 0b1;
                self.tick();
                let adjustment = if z_sign { 0xFF } else { 0x00 };
                let result_msb = msb.wrapping_add(adjustment).wrapping_add(carry.into());
                self.tick();
                self.registers.sp = u16::from_le_bytes([result_lsb, result_msb]);
            }
            Operation::LoadStackOffset(offset) => {
                let [lsb, msb] = self.registers.sp.to_le_bytes();
                let (result_lsb, carry) = lsb.overflowing_add(offset as u8);
                self.registers.l = result_lsb;
                self.registers
                    .f
                    .set_zero(false)
                    .set_subtract(false)
                    .set_half_carry((lsb & 0xF).wrapping_add(offset as u8 & 0xF) & 0x10 == 0x10)
                    .set_carry(carry);
                let z_sign = (result_lsb >> 7) & 0b1 == 0b1;
                self.tick();
                let adjustment = if z_sign { 0xFF } else { 0x00 };
                let result_msb = msb
                    .wrapping_add(adjustment)
                    .wrapping_add(self.registers.f.carry().into());
                self.registers.h = result_msb;
            }
            Operation::DisableInterrupt => {
                self.ime = false;
            }
            Operation::EnableInterrupt => {
                self.ime = true;
            }
            Operation::Call(condition, target) => {
                let condition_met = match condition {
                    Condition::None => true,
                    Condition::NZ => !self.registers.f.zero(),
                    Condition::Z => self.registers.f.zero(),
                    Condition::NC => !self.registers.f.carry(),
                    Condition::C => self.registers.f.carry(),
                };
                let [lsb, msb] = self.pc.to_le_bytes();
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.tick();
                self.memory.set_u8(self.registers.sp, msb);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.tick();
                self.memory.set_u8(self.registers.sp, lsb);
                let Target::Imm16(address) = target else {
                    unimplemented!("Invalid target for operation");
                };
                self.pc = address;
                self.tick();
            }
            Operation::Restart(address) => {
                let [lsb, msb] = self.pc.to_le_bytes();
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.tick();
                self.memory.set_u8(self.registers.sp, msb);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.tick();
                self.memory.set_u8(self.registers.sp, lsb);
                self.pc = address;
                self.tick();
            }
            Operation::Rotate(rotation_type, direction, target) => todo!(),
            Operation::ShiftArithmetic(direction, target) => todo!(),
            Operation::Swap(target) => todo!(),
            Operation::ShiftRightLogical(target) => todo!(),
            Operation::TestBit(_, target) => todo!(),
            Operation::ResetBit(_, target) => todo!(),
            Operation::SetBit(_, target) => todo!(),
        }
    }

    fn halt(&self) {
        todo!()
    }

    fn load(&mut self, destination: Target, source: Target) {
        match source {
            Target::R8(register) => self.load8(destination, self.read_register8(register)),
            Target::Imm8(value) => self.load8(destination, value),
            Target::R16(register) => self.load16(destination, self.read_register16(register)),
            Target::Imm16(value) => self.load16(destination, value),
            Target::Ind(indirect) => match indirect {
                Indirect::R16(register) => {
                    self.tick();
                    let value = self.memory.read_u8(self.read_register16(register));
                    self.load8(destination, value)
                }
                Indirect::Imm16(value) => {
                    self.tick();
                    let value = self.memory.read_u8(value);
                    self.load8(destination, value)
                }
                Indirect::HLI => {
                    self.tick();
                    let value = self.memory.read_u8(self.registers.hl());
                    self.registers.set_hl(self.registers.hl().wrapping_add(1));
                    self.load8(destination, value)
                }
                Indirect::HLD => {
                    self.tick();
                    let value = self.memory.read_u8(self.registers.hl());
                    self.registers.set_hl(self.registers.hl().wrapping_sub(1));
                    self.load8(destination, value)
                }
                Indirect::C => {
                    self.tick();
                    let value = self
                        .memory
                        .read_u8(u16::from_le_bytes([self.registers.c, 0xFF]));
                    self.load8(destination, value)
                }
            },
        };
    }
    fn load8(&mut self, destination: Target, value: u8) {
        match destination {
            Target::R8(register) => {
                self.set_register8(register, value);
            }
            Target::R16(_) => unimplemented!("Invalid 8-bit load to 16-bit register"),
            Target::Imm8(_) => unimplemented!("Invalid 8-bit load to immediate value"),
            Target::Imm16(_) => unimplemented!("Invalid 8-bit load to immediate value"),
            Target::Ind(indirect) => match indirect {
                Indirect::R16(register) => {
                    self.memory.set_u8(self.read_register16(register), value);
                    self.tick();
                }
                Indirect::Imm16(address) => {
                    self.memory.set_u8(address, value);
                    self.tick();
                }
                Indirect::HLI => {
                    self.memory.set_u8(self.registers.hl(), value);
                    self.registers.set_hl(self.registers.hl().wrapping_add(1));
                    self.tick();
                }
                Indirect::HLD => {
                    self.memory.set_u8(self.registers.hl(), value);
                    self.registers.set_hl(self.registers.hl().wrapping_sub(1));
                    self.tick();
                }
                Indirect::C => {
                    let addr = u16::from_le_bytes([self.registers.c, 0xFF]);
                    self.memory.set_u8(addr, value);
                    self.tick();
                }
            },
        }
    }

    fn load16(&mut self, destination: Target, value: u16) {
        match destination {
            Target::R8(_) => unimplemented!("Invalid 16-bit write to 8-bit register"),
            Target::R16(register) => {
                self.set_register16(register, value);
            }
            Target::Imm8(_) => unimplemented!("Invalid write to immediate"),
            Target::Imm16(_) => unimplemented!("Invalid write to immediate"),
            Target::Ind(indirect) => match indirect {
                Indirect::R16(_) => unimplemented!("No indirect 16-bit writes to registers"),
                Indirect::Imm16(address) => {
                    let [lsb, msb] = address.to_le_bytes();
                    self.memory.set_u8(address, lsb);
                    self.tick();
                    self.memory.set_u8(address.wrapping_add(1), msb);
                    self.tick()
                }
                _ => unimplemented!("Invalid 16-bit write requested"),
            },
        }
    }

    fn read_register8(&self, register: Registers8) -> u8 {
        match register {
            Registers8::A => self.registers.a,
            Registers8::B => self.registers.b,
            Registers8::C => self.registers.c,
            Registers8::D => self.registers.d,
            Registers8::E => self.registers.e,
            Registers8::H => self.registers.h,
            Registers8::L => self.registers.l,
        }
    }

    fn read_register16(&self, register: Registers16) -> u16 {
        match register {
            Registers16::BC => self.registers.bc(),
            Registers16::DE => self.registers.de(),
            Registers16::HL => self.registers.hl(),
            Registers16::SP => self.registers.sp,
            Registers16::AF => self.registers.af(),
        }
    }
    fn set_register16(&mut self, register: Registers16, value: u16) {
        match register {
            Registers16::BC => self.registers.set_bc(value),
            Registers16::DE => self.registers.set_de(value),
            Registers16::HL => self.registers.set_hl(value),
            Registers16::SP => self.registers.sp = value,
            Registers16::AF => self.registers.set_af(value),
        }
    }

    fn set_register8(&mut self, register: Registers8, value: u8) {
        match register {
            Registers8::A => self.registers.a == value,
            Registers8::B => self.registers.b == value,
            Registers8::C => self.registers.c == value,
            Registers8::D => self.registers.d == value,
            Registers8::E => self.registers.e == value,
            Registers8::H => self.registers.h == value,
            Registers8::L => self.registers.l == value,
        };
    }

    fn increment(&mut self, target: Target) {
        match target {
            Target::R8(register) => {
                let current = self.read_register8(register);
                let (result, _carry) = current.overflowing_add(1);
                let half_carry = current & 0x0F == 0x0F;
                self.set_register8(register, result);
                self.registers
                    .f
                    .set_zero(result == 0)
                    .set_subtract(false)
                    .set_half_carry(half_carry);
            }
            Target::R16(register) => {
                self.set_register16(register, self.read_register16(register).wrapping_add(1));
                self.tick();
            }
            Target::Ind(indirect) => match indirect {
                Indirect::R16(register) => {
                    let address = self.read_register16(register);
                    let value = self.memory.read_u8(address);
                    self.tick();
                    let (result, _carry) = value.overflowing_add(1);
                    let half_carry = value & 0x0F == 0x0F;
                    self.memory.set_u8(address, result);
                    self.registers
                        .f
                        .set_zero(result == 0)
                        .set_subtract(false)
                        .set_half_carry(half_carry);
                    self.tick();
                }
                _ => unimplemented!("Invalid target for operation"),
            },
            _ => unimplemented!("Invalid target for operation"),
        }
    }

    fn decrement(&mut self, target: Target) {
        match target {
            Target::R8(register) => {
                let current = self.read_register8(register);
                let result = current.wrapping_sub(1);
                let half_carry = current & 0x0F == 0;
                self.set_register8(register, result);
                self.registers
                    .f
                    .set_zero(result == 0)
                    .set_subtract(true)
                    .set_half_carry(half_carry);
            }
            Target::R16(register) => {
                self.set_register16(register, self.read_register16(register).wrapping_sub(1));
                self.tick();
            }
            Target::Ind(indirect) => match indirect {
                Indirect::R16(register) => {
                    let address = self.read_register16(register);
                    let value = self.memory.read_u8(address);
                    self.tick();
                    let result = value.wrapping_sub(1);
                    let half_carry = value & 0x0F == 0;
                    self.memory.set_u8(address, result);
                    self.registers
                        .f
                        .set_zero(result == 0)
                        .set_subtract(true)
                        .set_half_carry(half_carry);
                    self.tick();
                }
                _ => unimplemented!("Invalid target for operation"),
            },
            _ => unimplemented!("Invalid target for operation"),
        }
    }

    fn stop(&self) {
        todo!()
    }

    fn jump(&mut self, condition: Condition, target: Target) {
        match target {
            Target::R16(register) => self.pc = self.read_register16(register),
            Target::Imm16(address) => match condition {
                Condition::None => {
                    self.pc = address;
                    self.tick();
                }
                Condition::NZ => {
                    if !self.registers.f.zero() {
                        self.pc = address;
                        self.tick();
                    }
                }
                Condition::Z => {
                    if self.registers.f.zero() {
                        self.pc = address;
                        self.tick();
                    }
                }
                Condition::NC => {
                    if !self.registers.f.carry() {
                        self.pc = address;
                        self.tick();
                    }
                }
                Condition::C => {
                    if self.registers.f.carry() {
                        self.pc = address;
                        self.tick();
                    }
                }
            },
            _ => unimplemented!("Invalid target for operation"),
        }
    }

    fn jump_relative(&mut self, condition: Condition, offset: i8) {
        let condition_met = match condition {
            Condition::None => true,
            Condition::NZ => !self.registers.f.zero(),
            Condition::Z => self.registers.f.zero(),
            Condition::NC => !self.registers.f.carry(),
            Condition::C => self.registers.f.carry(),
        };
        if condition_met {
            self.tick();
            let result = self.pc.wrapping_add_signed(offset as i16);
            self.tick();
            self.pc = result;
        }
        self.tick();
    }

    fn rotate_accumulator(&mut self, rotation_type: RotationType, direction: Direction) {
        let (result, carry) = match (rotation_type, direction) {
            (RotationType::Circular, Direction::Left) => {
                let b7 = (self.registers.a >> 7) & 0b1 == 1;
                (self.registers.a.rotate_left(1), b7)
            }
            (RotationType::Circular, Direction::Right) => {
                let b0 = self.registers.a & 0b1 == 1;
                (self.registers.a.rotate_right(1), b0)
            }
            (RotationType::NonCircular, Direction::Left) => {
                let b7 = (self.registers.a >> 7) & 0b1 == 1;
                (
                    self.registers.a.shl(1) | self.registers.f.carry().conv::<u8>(),
                    b7,
                )
            }
            (RotationType::NonCircular, Direction::Right) => {
                let b0 = (self.registers.a >> 7) & 0b1 == 1;
                (
                    self.registers.a.shr(1) | (self.registers.f.carry().conv::<u8>() << 7),
                    b0,
                )
            }
        };
        self.registers.a = result;
        self.registers
            .f
            .set_zero(false)
            .set_subtract(false)
            .set_half_carry(false)
            .set_carry(carry);
    }
}

fn decompose_octal_triplet(value: u8) -> (u8, u8, u8) {
    ((value >> 6) & 0o7, (value >> 3) & 0o7, value & 0o7)
}

fn decompose_octal_cb(value: u8) -> (u8, u8) {
    ((value >> 3) & 0o77, value & 0o7)
}
