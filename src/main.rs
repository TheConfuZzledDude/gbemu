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
    Compare(Target, Target),
    Or(Target, Target),
    Xor(Target, Target),
    And(Target, Target),
    Sbc(Target, Target),
    Sub(Target, Target),
    Adc(Target, Target),
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

    fn fetch(&mut self) {
        use Registers8::*;
        use Registers16::*;
        use Target::*;
        let operation = match decompose_octal_triplet(self.ir) {
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
                    1 => Operation::Adc(R8(A), target),
                    2 => Operation::Sub(R8(A), target),
                    3 => Operation::Sbc(R8(A), target),
                    4 => Operation::And(R8(A), target),
                    5 => Operation::Xor(R8(A), target),
                    6 => Operation::Or(R8(A), target),
                    7 => Operation::Compare(R8(A), target),
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
                self.tick();
                let offset = self.memory.read_u8(self.pc);
                let address = u16::from_le_bytes([offset, 0xFF]);
                match kind {
                    0o4 => Operation::Load(Ind(Indirect::Imm16(address)), R8(A)),
                    0o6 => Operation::Load(R8(A), Ind(Indirect::Imm16(address))),
                    _ => unreachable!(),
                }
            }
            (0o3, 0o5, 0o0) => {
                self.tick();
                let offset = self.memory.read_u8(self.pc) as i8;
                Operation::AddStack(offset)
            }
            (0o3, 0o7, 0o0) => {
                self.tick();
                let offset = self.memory.read_u8(self.pc) as i8;
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
                self.tick();
                let lsb = self.memory.read_u8(self.pc);
                self.tick();
                let msb = self.memory.read_u8(self.pc);
                let address = u16::from_le_bytes([lsb, msb]);
                Operation::Jump(condition, Imm16(address))
            }
            (0o3, op @ 0o4..=0o7, 0o2) => {
                let dest = if op & 1 == 0 {
                    Ind(Indirect::C)
                } else {
                    self.tick();
                    let lsb = self.memory.read_u8(self.pc);
                    self.tick();
                    let msb = self.memory.read_u8(self.pc);
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
                self.tick();
                let lsb = self.memory.read_u8(self.pc);
                self.tick();
                let msb = self.memory.read_u8(self.pc);
                let address = u16::from_le_bytes([lsb, msb]);
                Operation::Jump(Condition::None, Imm16(address))
            }
            (0o3, 0o1, 0o3) => {
                self.tick();
                self.fetch_cb_operation()
            }
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
                self.tick();
                let lsb = self.memory.read_u8(self.pc);
                self.tick();
                let msb = self.memory.read_u8(self.pc);
                let address = u16::from_le_bytes([lsb, msb]);
                Operation::Call(condition, Imm16(address))
            }
            (0o3, 0o4..=0o7, 0o4) => {
                error!("Invalid opcode");
                todo!("Decide what to do on invalid opcode")
            }
            (0o3, 0o1, 0o5) => {
                self.tick();
                let lsb = self.memory.read_u8(self.pc);
                self.tick();
                let msb = self.memory.read_u8(self.pc);
                let address = u16::from_le_bytes([lsb, msb]);
                Operation::Call(Condition::None, Imm16(address))
            }
            (0o3, 0o3 | 0o5 | 0o7, 0o5) => {
                error!("Invalid opcode");
                todo!("Decide what to do on invalid opcode")
            }
            (0o3, kind @ 0o0..=0o7, 0o6) => {
                self.tick();
                let value = self.memory.read_u8(self.pc);
                let dest = R8(A);
                let operation = match kind {
                    0o0 => Operation::Add,
                    0o1 => Operation::Adc,
                    0o2 => Operation::Sub,
                    0o3 => Operation::Sbc,
                    0o4 => Operation::And,
                    0o5 => Operation::Xor,
                    0o6 => Operation::Or,
                    0o7 => Operation::Compare,
                    _ => unreachable!(),
                };
                operation(dest, Target::Imm8(value))
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

    fn fetch_cb_operation(&self) -> Operation {
        use Registers8::*;
        use Registers16::*;
        use Target::*;
        let (operation, target) = decompose_octal_cb(self.memory.read_u8(self.pc));

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
            Operation::Nop => todo!(),
            Operation::Halt => todo!(),
            Operation::Load(target, target1) => todo!(),
            Operation::Inc(target) => todo!(),
            Operation::Dec(target) => todo!(),
            Operation::Stop => todo!(),
            Operation::JumpRelative(_) => todo!(),
            Operation::RotateAccumulator(rotation_type, direction) => todo!(),
            Operation::ComplementCarry => todo!(),
            Operation::SetCarry => todo!(),
            Operation::ComplementAccumulator => todo!(),
            Operation::DecimalAdjustAccumulator => todo!(),
            Operation::Compare(target, target1) => todo!(),
            Operation::Or(target, target1) => todo!(),
            Operation::Xor(target, target1) => todo!(),
            Operation::And(target, target1) => todo!(),
            Operation::Sbc(target, target1) => todo!(),
            Operation::Sub(target, target1) => todo!(),
            Operation::Adc(target, target1) => todo!(),
            Operation::Add(destination, source) => todo!(),
            Operation::Return(condition) => todo!(),
            Operation::Push(registers16) => todo!(),
            Operation::Pop(registers16) => todo!(),
            Operation::ReturnInterrupt => todo!(),
            Operation::Jump(condition, target) => todo!(),
            Operation::AddStack(_) => todo!(),
            Operation::LoadStackOffset(_) => todo!(),
            Operation::DisableInterrupt => todo!(),
            Operation::EnableInterrupt => todo!(),
            Operation::Call(condition, target) => todo!(),
            Operation::Restart(_) => todo!(),
            Operation::Rotate(rotation_type, direction, target) => todo!(),
            Operation::ShiftArithmetic(direction, target) => todo!(),
            Operation::Swap(target) => todo!(),
            Operation::ShiftRightLogical(target) => todo!(),
            Operation::TestBit(_, target) => todo!(),
            Operation::ResetBit(_, target) => todo!(),
            Operation::SetBit(_, target) => todo!(),
        }
    }
}

fn decompose_octal_triplet(value: u8) -> (u8, u8, u8) {
    ((value >> 6) & 0o7, (value >> 3) & 0o7, value & 0o7)
}

fn decompose_octal_cb(value: u8) -> (u8, u8) {
    ((value >> 3) & 0o77, value & 0o7)
}
