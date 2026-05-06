use better_default::Default;
use tap::Conv;
use tracing::{debug, error, info};

use crate::context::{self, Context};

pub(crate) mod registers;

pub(crate) enum Operation {
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
pub(crate) enum Registers8 {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Registers16 {
    BC,
    DE,
    HL,
    SP,
    AF,
}

#[derive(Debug, Clone, Copy)]

pub(crate) enum Indirect {
    R16(Registers16),
    Imm16(u16),
    HLI,
    HLD,
    C,
}

#[derive(Debug, Clone)]

pub(crate) enum Target {
    R8(Registers8),
    R16(Registers16),
    Imm8(u8),
    Imm16(u16),
    Ind(Indirect),
}

#[derive(Debug, Clone, Copy)]

pub(crate) enum Condition {
    None,
    NZ,
    Z,
    NC,
    C,
}

#[derive(Debug, Clone, Copy)]

pub(crate) enum RotationType {
    Circular,
    NonCircular,
}

#[derive(Debug, Clone, Copy)]

pub(crate) enum Direction {
    Left,
    Right,
}

#[derive(Default)]
pub(crate) struct CPU {
    pub(crate) registers: registers::Registers,
    pub(crate) pc: u16,
    pub(crate) ir: u8,
    pub(crate) ime: bool,
    pub(crate) halted: bool,
}

impl CPU {
    pub(crate) fn tick(&mut self, ctx: &mut Context) {
        ctx.memory.io.timer.tima_written = false;
        let tima_overflow = ctx.memory.io.timer.tima_overflow;
        ctx.memory.io.timer.tima_overflow = None;
        ctx.memory.io.timer.clock_tick();
        if let Some(tma) = tima_overflow {
            ctx.memory
                .io
                .timer
                .handle_overflow(tma, &mut ctx.memory.io.interrupt);
        }
    }
    pub(crate) fn tick_and_increment_pc(&mut self, ctx: &mut Context) {
        self.tick(ctx);
        self.pc = self.pc.wrapping_add(1);
    }

    pub(crate) fn load_boot_rom(&mut self, rom: &[u8], ctx: &mut Context) {
        ctx.memory.rom[..rom.len()].copy_from_slice(rom);
    }

    pub(crate) fn load_rom(&mut self, rom: &[u8], ctx: &mut Context) {
        ctx.memory.rom.copy_from_slice(&rom[..1024 * 16]);
        ctx.memory.rom_banks[0].copy_from_slice(&rom[1024 * 16..]);
    }

    pub(crate) fn load_debug_initial_state(&mut self, _ctx: &mut Context) {
        self.registers.a = 0x01;
        *self.registers.f = 0xB0;
        self.registers.b = 0x00;
        self.registers.c = 0x13;
        self.registers.d = 0x00;
        self.registers.e = 0xD8;
        self.registers.h = 0x01;
        self.registers.l = 0x4D;
        self.registers.sp = 0xFFFE;
        self.pc = 0x0100;
    }

    pub(crate) fn run(&mut self, ctx: &mut Context) {
        debug!(
            "A:{:02X} F:{:02X} B:{:02X} C:{:02X} D:{:02X} E:{:02X} H:{:02X} L:{:02X} SP:{:04X} PC:{:04X} PCMEM:{:02X},{:02X},{:02X},{:02X}",
            self.registers.a,
            *self.registers.f,
            self.registers.b,
            self.registers.c,
            self.registers.d,
            self.registers.e,
            self.registers.h,
            self.registers.l,
            self.registers.sp,
            self.pc,
            ctx.memory.read_u8(self.pc),
            ctx.memory.read_u8(self.pc + 1),
            ctx.memory.read_u8(self.pc + 2),
            ctx.memory.read_u8(self.pc + 3),
        );
        self.ir = ctx.memory.read_u8(self.pc);
        self.pc = self.pc.wrapping_add(1);
        loop {
            if self.halted {
                if (ctx.memory.io.interrupt.interrupt_flag & ctx.memory.ie) != 0 {
                    self.halted = false;
                    self.tick_and_increment_pc(ctx);
                } else {
                    self.tick(ctx);
                    continue;
                }
            }

            if (ctx.memory.io.interrupt.interrupt_flag & ctx.memory.ie) != 0 && self.ime {
                self.handle_interrupts(ctx);
            } else if !self.halted {
                let operation = self.decode(ctx);
                self.execute_operation(operation, ctx);
                debug!(
                    "A:{:02X} F:{:02X} B:{:02X} C:{:02X} D:{:02X} E:{:02X} H:{:02X} L:{:02X} SP:{:04X} PC:{:04X} PCMEM:{:02X},{:02X},{:02X},{:02X}",
                    self.registers.a,
                    *self.registers.f,
                    self.registers.b,
                    self.registers.c,
                    self.registers.d,
                    self.registers.e,
                    self.registers.h,
                    self.registers.l,
                    self.registers.sp,
                    self.pc,
                    ctx.memory.read_u8(self.pc),
                    ctx.memory.read_u8(self.pc + 1),
                    ctx.memory.read_u8(self.pc + 2),
                    ctx.memory.read_u8(self.pc + 3),
                );
            }

            if !self.halted {
                self.ir = ctx.memory.read_u8(self.pc);
                self.tick_and_increment_pc(ctx);
            } else {
                self.ir = ctx.memory.read_u8(self.pc);
                self.tick(ctx);
            }
        }
    }

    pub(crate) fn handle_interrupts(&mut self, ctx: &mut Context) {
        let masked = ctx.memory.io.interrupt.interrupt_flag & ctx.memory.ie;
        let next_interrupt = if masked & 0b1 != 0 {
            context::InterruptType::VBlank
        } else if masked & 0b10 != 0 {
            context::InterruptType::LCD
        } else if masked & 0b100 != 0 {
            context::InterruptType::Timer
        } else if masked & 0b1000 != 0 {
            context::InterruptType::Serial
        } else if masked & 0b10000 != 0 {
            context::InterruptType::Joypad
        } else {
            panic!()
        };
        let address = match next_interrupt {
            context::InterruptType::Joypad => 0x0060,
            context::InterruptType::Serial => 0x0058,
            context::InterruptType::Timer => 0x0050,
            context::InterruptType::LCD => 0x0048,
            context::InterruptType::VBlank => 0x0040,
        };
        ctx.memory.io.interrupt.clear_interrupt(next_interrupt);
        self.ime = false;
        self.pc = self.pc.wrapping_sub(1);
        self.tick(ctx);
        self.tick(ctx);
        self.execute_operation(
            Operation::Call(Condition::None, Target::Imm16(address)),
            ctx,
        );
    }

    pub(crate) fn decode(&mut self, ctx: &mut Context) -> Operation {
        use Registers8::*;
        use Registers16::*;
        use Target::*;
        match decompose_octal_triplet(self.ir) {
            // https://gbdev.io/gb-opcodes/optables/octal
            (0o0, 0o0, 0o0) => Operation::Nop,
            (0o0, 0o1, 0o0) => {
                let lsb = ctx.memory.read_u8(self.pc);
                self.tick_and_increment_pc(ctx);
                let msb = ctx.memory.read_u8(self.pc);
                self.tick_and_increment_pc(ctx);
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

                let offset = ctx.memory.read_u8(self.pc) as i8;
                self.tick_and_increment_pc(ctx);
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
                    let lsb = ctx.memory.read_u8(self.pc);
                    self.tick_and_increment_pc(ctx);
                    let msb = ctx.memory.read_u8(self.pc);
                    self.tick_and_increment_pc(ctx);
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
                let value = ctx.memory.read_u8(self.pc);
                self.tick_and_increment_pc(ctx);
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
            (0o1, destination @ 0o0..=0o7, source) => {
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
                let offset = ctx.memory.read_u8(self.pc);
                self.tick_and_increment_pc(ctx);
                let address = u16::from_le_bytes([offset, 0xFF]);
                match kind {
                    0o4 => Operation::Load(Ind(Indirect::Imm16(address)), R8(A)),
                    0o6 => Operation::Load(R8(A), Ind(Indirect::Imm16(address))),
                    _ => unreachable!(),
                }
            }
            (0o3, 0o5, 0o0) => {
                let offset = ctx.memory.read_u8(self.pc) as i8;
                self.tick_and_increment_pc(ctx);
                Operation::AddStack(offset)
            }
            (0o3, 0o7, 0o0) => {
                let offset = ctx.memory.read_u8(self.pc) as i8;
                self.tick_and_increment_pc(ctx);
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
                let lsb = ctx.memory.read_u8(self.pc);
                self.tick_and_increment_pc(ctx);
                let msb = ctx.memory.read_u8(self.pc);
                self.tick_and_increment_pc(ctx);
                let address = u16::from_le_bytes([lsb, msb]);
                Operation::Jump(condition, Imm16(address))
            }
            (0o3, op @ 0o4..=0o7, 0o2) => {
                let dest = if op & 1 == 0 {
                    Ind(Indirect::C)
                } else {
                    let lsb = ctx.memory.read_u8(self.pc);
                    self.tick_and_increment_pc(ctx);
                    let msb = ctx.memory.read_u8(self.pc);
                    self.tick_and_increment_pc(ctx);
                    let address = u16::from_le_bytes([lsb, msb]);
                    Ind(Indirect::Imm16(address))
                };
                let source = R8(A);
                match op {
                    0o4..=0o5 => Operation::Load(dest, source),
                    0o6..=0o7 => Operation::Load(source, dest),
                    _ => unreachable!(),
                }
            }
            (0o3, 0o0, 0o3) => {
                let lsb = ctx.memory.read_u8(self.pc);
                self.tick_and_increment_pc(ctx);
                let msb = ctx.memory.read_u8(self.pc);
                self.tick_and_increment_pc(ctx);
                let address = u16::from_le_bytes([lsb, msb]);
                Operation::Jump(Condition::None, Imm16(address))
            }
            (0o3, 0o1, 0o3) => self.fetch_cb_operation(ctx),
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
                let lsb = ctx.memory.read_u8(self.pc);
                self.tick_and_increment_pc(ctx);
                let msb = ctx.memory.read_u8(self.pc);
                self.tick_and_increment_pc(ctx);
                let address = u16::from_le_bytes([lsb, msb]);
                Operation::Call(condition, Imm16(address))
            }
            (0o3, 0o4..=0o7, 0o4) => {
                error!("Invalid opcode");
                todo!("Decide what to do on invalid opcode")
            }
            (0o3, 0o1, 0o5) => {
                let lsb = ctx.memory.read_u8(self.pc);
                self.tick_and_increment_pc(ctx);
                let msb = ctx.memory.read_u8(self.pc);
                self.tick_and_increment_pc(ctx);
                let address = u16::from_le_bytes([lsb, msb]);
                Operation::Call(Condition::None, Imm16(address))
            }
            (0o3, 0o3 | 0o5 | 0o7, 0o5) => {
                error!("Invalid opcode");
                todo!("Decide what to do on invalid opcode")
            }
            (0o3, kind @ 0o0..=0o7, 0o6) => {
                let value = ctx.memory.read_u8(self.pc);
                self.tick_and_increment_pc(ctx);
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
        }
    }

    pub(crate) fn fetch_cb_operation(&mut self, ctx: &mut Context) -> Operation {
        use Registers8::*;
        use Registers16::*;
        use Target::*;
        let (operation, target) = decompose_octal_cb(ctx.memory.read_u8(self.pc));
        self.tick_and_increment_pc(ctx);

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

        match operation {
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
        }
    }

    pub(crate) fn execute_operation(&mut self, operation: Operation, ctx: &mut Context) {
        match operation {
            Operation::Nop => {}
            Operation::Halt => self.halt(ctx),
            Operation::Load(destination, source) => self.load(destination, source, ctx),
            Operation::Inc(target) => self.increment(target, ctx),
            Operation::Dec(target) => self.decrement(target, ctx),
            Operation::Stop => self.stop(ctx),
            Operation::Jump(condition, target) => self.jump(condition, target, ctx),
            Operation::JumpRelative(condition, offset) => {
                self.jump_relative(condition, offset, ctx)
            }
            Operation::RotateAccumulator(rotation_type, direction) => {
                self.rotate_accumulator(rotation_type, direction, ctx)
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
                        let value = ctx.memory.read_u8(self.read_register16(register));
                        self.tick(ctx);
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
                        let value = ctx.memory.read_u8(self.read_register16(register));
                        self.tick(ctx);
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
                        let value = ctx.memory.read_u8(self.read_register16(register));
                        self.tick(ctx);
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
                        let value = ctx.memory.read_u8(self.read_register16(register));
                        self.tick(ctx);
                        value
                    }
                    _ => unimplemented!("Invalid targets for operation"),
                };
                let result = self.registers.a & value;
                self.registers
                    .f
                    .set_zero(result == 0)
                    .set_subtract(false)
                    .set_half_carry(true)
                    .set_carry(false);
                self.registers.a = result;
            }
            Operation::Sbc(target) => {
                let value = match target {
                    Target::R8(register) => self.read_register8(register),
                    Target::Imm8(value) => value,
                    Target::Ind(Indirect::R16(register)) => {
                        let value = ctx.memory.read_u8(self.read_register16(register));
                        self.tick(ctx);
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
                        let value = ctx.memory.read_u8(self.read_register16(register));
                        self.tick(ctx);
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
                        let value = ctx.memory.read_u8(self.read_register16(register));
                        self.tick(ctx);
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
                            let value = ctx.memory.read_u8(self.read_register16(register));
                            self.tick(ctx);
                            value
                        }
                        _ => unimplemented!("Invalid targets for operation"),
                    };

                    let (result, carry) = self.registers.a.overflowing_add(value);

                    self.registers
                        .f
                        .set_zero(result == 0)
                        .set_subtract(false)
                        .set_half_carry(
                            (self.registers.a & 0xF).wrapping_add(value & 0xF) & 0x10 == 0x10,
                        )
                        .set_carry(carry);
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
                        self.tick(ctx);
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
                        self.tick(ctx)
                    };
                    let lsb = ctx.memory.read_u8(self.registers.sp);
                    self.registers.sp = self.registers.sp.wrapping_add(1);
                    self.tick(ctx);
                    let msb = ctx.memory.read_u8(self.registers.sp);
                    self.registers.sp = self.registers.sp.wrapping_add(1);
                    self.tick(ctx);
                    self.pc = u16::from_le_bytes([lsb, msb]);
                }
                self.tick(ctx);
            }
            Operation::Push(register) => {
                let [lsb, msb] = self.read_register16(register).to_le_bytes();
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.tick(ctx);
                ctx.memory.set_u8(self.registers.sp, msb);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.tick(ctx);
                ctx.memory.set_u8(self.registers.sp, lsb);
                self.tick(ctx);
            }
            Operation::Pop(register) => {
                let lsb = ctx.memory.read_u8(self.registers.sp);
                self.registers.sp = self.registers.sp.wrapping_add(1);
                self.tick(ctx);
                let msb = ctx.memory.read_u8(self.registers.sp);
                self.registers.sp = self.registers.sp.wrapping_add(1);
                self.tick(ctx);
                self.set_register16(register, u16::from_le_bytes([lsb, msb]));
            }
            Operation::ReturnInterrupt => {
                self.tick(ctx);
                let lsb = ctx.memory.read_u8(self.registers.sp);
                self.registers.sp = self.registers.sp.wrapping_add(1);
                self.tick(ctx);
                let msb = ctx.memory.read_u8(self.registers.sp);
                self.registers.sp = self.registers.sp.wrapping_add(1);
                self.tick(ctx);
                self.pc = u16::from_le_bytes([lsb, msb]);
                self.ime = true;
                self.tick(ctx);
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
                self.tick(ctx);
                let adjustment = if z_sign { 0xFF } else { 0x00 };
                let result_msb = msb.wrapping_add(adjustment).wrapping_add(carry.into());
                self.tick(ctx);
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
                self.tick(ctx);
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
                if condition_met {
                    let [lsb, msb] = self.pc.to_le_bytes();
                    self.registers.sp = self.registers.sp.wrapping_sub(1);
                    self.tick(ctx);
                    ctx.memory.set_u8(self.registers.sp, msb);
                    self.registers.sp = self.registers.sp.wrapping_sub(1);
                    self.tick(ctx);
                    ctx.memory.set_u8(self.registers.sp, lsb);
                    let Target::Imm16(address) = target else {
                        unimplemented!("Invalid target for operation");
                    };
                    self.pc = address;
                }
                self.tick(ctx);
            }
            Operation::Restart(address) => {
                let [lsb, msb] = self.pc.to_le_bytes();
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.tick(ctx);
                ctx.memory.set_u8(self.registers.sp, msb);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.tick(ctx);
                ctx.memory.set_u8(self.registers.sp, lsb);
                self.pc = address;
                self.tick(ctx);
            }
            Operation::Rotate(rotation_type, direction, target) => {
                self.rotate(rotation_type, direction, target, ctx)
            }
            Operation::ShiftArithmetic(direction, target) => {
                let prev_value = match target {
                    Target::R8(register) => self.read_register8(register),
                    Target::Ind(Indirect::R16(Registers16::HL)) => {
                        let value = ctx.memory.read_u8(self.registers.hl());
                        self.tick(ctx);
                        value
                    }
                    _ => unimplemented!("Invalid target for operation"),
                };

                let (result, carry) = match direction {
                    Direction::Left => (prev_value << 1, (prev_value >> 7) & 0b1 == 0b1),
                    Direction::Right => (
                        (prev_value >> 1) | (prev_value & 0b1000_0000),
                        prev_value & 0b1 == 0b1,
                    ),
                };

                self.registers
                    .f
                    .set_zero(result == 0)
                    .set_subtract(false)
                    .set_half_carry(false)
                    .set_carry(carry);
                match target {
                    Target::R8(register) => {
                        self.set_register8(register, result);
                    }
                    Target::Ind(Indirect::R16(Registers16::HL)) => {
                        ctx.memory.set_u8(self.registers.hl(), result);
                        self.tick(ctx);
                    }
                    _ => unimplemented!("Invalid target for operation"),
                }
            }
            Operation::Swap(target) => {
                let prev_value = match target {
                    Target::R8(register) => self.read_register8(register),
                    Target::Ind(Indirect::R16(Registers16::HL)) => {
                        let value = ctx.memory.read_u8(self.registers.hl());
                        self.tick(ctx);
                        value
                    }
                    _ => unimplemented!("Invalid target for operation"),
                };

                let low = prev_value & 0b1111;
                let high = (prev_value >> 4) & 0b1111;
                let result = (low << 4) | high;

                self.registers
                    .f
                    .set_zero(result == 0)
                    .set_subtract(false)
                    .set_half_carry(false)
                    .set_carry(false);
                match target {
                    Target::R8(register) => {
                        self.set_register8(register, result);
                    }
                    Target::Ind(Indirect::R16(Registers16::HL)) => {
                        ctx.memory.set_u8(self.registers.hl(), result);
                        self.tick(ctx);
                    }
                    _ => unimplemented!("Invalid target for operation"),
                }
            }
            Operation::ShiftRightLogical(target) => {
                let prev_value = match target {
                    Target::R8(register) => self.read_register8(register),
                    Target::Ind(Indirect::R16(Registers16::HL)) => {
                        let value = ctx.memory.read_u8(self.registers.hl());
                        self.tick(ctx);
                        value
                    }
                    _ => unimplemented!("Invalid target for operation"),
                };

                let result = prev_value >> 1;
                let carry = prev_value & 0b1 == 0b1;

                self.registers
                    .f
                    .set_zero(result == 0)
                    .set_subtract(false)
                    .set_half_carry(false)
                    .set_carry(carry);
                match target {
                    Target::R8(register) => {
                        self.set_register8(register, result);
                    }
                    Target::Ind(Indirect::R16(Registers16::HL)) => {
                        ctx.memory.set_u8(self.registers.hl(), result);
                        self.tick(ctx);
                    }
                    _ => unimplemented!("Invalid target for operation"),
                }
            }
            Operation::TestBit(bit, target) => {
                let prev_value = match target {
                    Target::R8(register) => self.read_register8(register),
                    Target::Ind(Indirect::R16(Registers16::HL)) => {
                        let value = ctx.memory.read_u8(self.registers.hl());
                        self.tick(ctx);
                        value
                    }
                    _ => unimplemented!("Invalid target for operation"),
                };
                self.registers
                    .f
                    .set_zero((prev_value >> bit) & 0b1 == 0)
                    .set_subtract(false)
                    .set_half_carry(true);
            }
            Operation::ResetBit(bit, target) => {
                let prev_value = match target {
                    Target::R8(register) => self.read_register8(register),
                    Target::Ind(Indirect::R16(Registers16::HL)) => {
                        let value = ctx.memory.read_u8(self.registers.hl());
                        self.tick(ctx);
                        value
                    }
                    _ => unimplemented!("Invalid target for operation"),
                };
                let result = prev_value & !(0b1 << (bit - 1));

                match target {
                    Target::R8(register) => {
                        self.set_register8(register, result);
                    }
                    Target::Ind(Indirect::R16(Registers16::HL)) => {
                        ctx.memory.set_u8(self.registers.hl(), result);
                        self.tick(ctx);
                    }
                    _ => unimplemented!("Invalid target for operation"),
                }
            }
            Operation::SetBit(bit, target) => {
                let prev_value = match target {
                    Target::R8(register) => self.read_register8(register),
                    Target::Ind(Indirect::R16(Registers16::HL)) => {
                        let value = ctx.memory.read_u8(self.registers.hl());
                        self.tick(ctx);
                        value
                    }
                    _ => unimplemented!("Invalid target for operation"),
                };
                let result = prev_value | (0b1 << (bit - 1));

                match target {
                    Target::R8(register) => {
                        self.set_register8(register, result);
                    }
                    Target::Ind(Indirect::R16(Registers16::HL)) => {
                        ctx.memory.set_u8(self.registers.hl(), result);
                        self.tick(ctx);
                    }
                    _ => unimplemented!("Invalid target for operation"),
                }
            }
        }
    }

    pub(crate) fn halt(&mut self, _ctx: &mut Context) {
        self.halted = false;
    }

    pub(crate) fn load(&mut self, destination: Target, source: Target, ctx: &mut Context) {
        // if self.pc == 0xC227 {
        //     debug!("BUGGED POINT: {:?} {:?}", destination, source)
        // }
        match source {
            Target::R8(register) => self.load8(destination, self.read_register8(register), ctx),
            Target::Imm8(value) => self.load8(destination, value, ctx),
            Target::R16(register) => self.load16(destination, self.read_register16(register), ctx),
            Target::Imm16(value) => self.load16(destination, value, ctx),
            Target::Ind(indirect) => match indirect {
                Indirect::R16(register) => {
                    self.tick(ctx);
                    let value = ctx.memory.read_u8(self.read_register16(register));
                    self.load8(destination, value, ctx)
                }
                Indirect::Imm16(value) => {
                    self.tick(ctx);
                    let value = ctx.memory.read_u8(value);
                    self.load8(destination, value, ctx)
                }
                Indirect::HLI => {
                    self.tick(ctx);
                    let value = ctx.memory.read_u8(self.registers.hl());
                    self.registers.set_hl(self.registers.hl().wrapping_add(1));
                    self.load8(destination, value, ctx)
                }
                Indirect::HLD => {
                    self.tick(ctx);
                    let value = ctx.memory.read_u8(self.registers.hl());
                    self.registers.set_hl(self.registers.hl().wrapping_sub(1));
                    self.load8(destination, value, ctx)
                }
                Indirect::C => {
                    self.tick(ctx);
                    let value = ctx
                        .memory
                        .read_u8(u16::from_le_bytes([self.registers.c, 0xFF]));
                    self.load8(destination, value, ctx)
                }
            },
        };
    }
    pub(crate) fn load8(&mut self, destination: Target, value: u8, ctx: &mut Context) {
        match destination {
            Target::R8(register) => {
                self.set_register8(register, value);
            }
            Target::R16(_) => unimplemented!("Invalid 8-bit load to 16-bit register"),
            Target::Imm8(_) => unimplemented!("Invalid 8-bit load to immediate value"),
            Target::Imm16(_) => unimplemented!("Invalid 8-bit load to immediate value"),
            Target::Ind(indirect) => match indirect {
                Indirect::R16(register) => {
                    ctx.memory.set_u8(self.read_register16(register), value);
                    self.tick(ctx);
                }
                Indirect::Imm16(address) => {
                    ctx.memory.set_u8(address, value);
                    self.tick(ctx);
                }
                Indirect::HLI => {
                    ctx.memory.set_u8(self.registers.hl(), value);
                    self.registers.set_hl(self.registers.hl().wrapping_add(1));
                    self.tick(ctx);
                }
                Indirect::HLD => {
                    ctx.memory.set_u8(self.registers.hl(), value);
                    self.registers.set_hl(self.registers.hl().wrapping_sub(1));
                    self.tick(ctx);
                }
                Indirect::C => {
                    let addr = u16::from_le_bytes([self.registers.c, 0xFF]);
                    ctx.memory.set_u8(addr, value);
                    self.tick(ctx);
                }
            },
        }
    }

    pub(crate) fn load16(&mut self, destination: Target, value: u16, ctx: &mut Context) {
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
                    ctx.memory.set_u8(address, lsb);
                    self.tick(ctx);
                    ctx.memory.set_u8(address.wrapping_add(1), msb);
                    self.tick(ctx)
                }
                _ => unimplemented!("Invalid 16-bit write requested"),
            },
        }
    }

    pub(crate) fn read_register8(&self, register: Registers8) -> u8 {
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

    pub(crate) fn read_register16(&self, register: Registers16) -> u16 {
        match register {
            Registers16::BC => self.registers.bc(),
            Registers16::DE => self.registers.de(),
            Registers16::HL => self.registers.hl(),
            Registers16::SP => self.registers.sp,
            Registers16::AF => self.registers.af(),
        }
    }
    pub(crate) fn set_register16(&mut self, register: Registers16, value: u16) {
        match register {
            Registers16::BC => self.registers.set_bc(value),
            Registers16::DE => self.registers.set_de(value),
            Registers16::HL => self.registers.set_hl(value),
            Registers16::SP => self.registers.sp = value,
            Registers16::AF => self.registers.set_af(value),
        }
    }

    pub(crate) fn set_register8(&mut self, register: Registers8, value: u8) {
        match register {
            Registers8::A => self.registers.a = value,
            Registers8::B => self.registers.b = value,
            Registers8::C => self.registers.c = value,
            Registers8::D => self.registers.d = value,
            Registers8::E => self.registers.e = value,
            Registers8::H => self.registers.h = value,
            Registers8::L => self.registers.l = value,
        };
    }

    pub(crate) fn increment(&mut self, target: Target, ctx: &mut Context) {
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
                self.tick(ctx);
            }
            Target::Ind(indirect) => match indirect {
                Indirect::R16(register) => {
                    let address = self.read_register16(register);
                    let value = ctx.memory.read_u8(address);
                    self.tick(ctx);
                    let (result, _carry) = value.overflowing_add(1);
                    let half_carry = value & 0x0F == 0x0F;
                    ctx.memory.set_u8(address, result);
                    self.registers
                        .f
                        .set_zero(result == 0)
                        .set_subtract(false)
                        .set_half_carry(half_carry);
                    self.tick(ctx);
                }
                _ => unimplemented!("Invalid target for operation"),
            },
            _ => unimplemented!("Invalid target for operation"),
        }
    }

    pub(crate) fn decrement(&mut self, target: Target, ctx: &mut Context) {
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
                self.tick(ctx);
            }
            Target::Ind(indirect) => match indirect {
                Indirect::R16(register) => {
                    let address = self.read_register16(register);
                    let value = ctx.memory.read_u8(address);
                    self.tick(ctx);
                    let result = value.wrapping_sub(1);
                    let half_carry = value & 0x0F == 0;
                    ctx.memory.set_u8(address, result);
                    self.registers
                        .f
                        .set_zero(result == 0)
                        .set_subtract(true)
                        .set_half_carry(half_carry);
                    self.tick(ctx);
                }
                _ => unimplemented!("Invalid target for operation"),
            },
            _ => unimplemented!("Invalid target for operation"),
        }
    }

    pub(crate) fn stop(&self, ctx: &mut Context) {
        todo!()
    }

    pub(crate) fn jump(&mut self, condition: Condition, target: Target, ctx: &mut Context) {
        match target {
            Target::R16(register) => self.pc = self.read_register16(register),
            Target::Imm16(address) => match condition {
                Condition::None => {
                    self.pc = address;
                    self.tick(ctx);
                }
                Condition::NZ => {
                    if !self.registers.f.zero() {
                        self.pc = address;
                        self.tick(ctx);
                    }
                }
                Condition::Z => {
                    if self.registers.f.zero() {
                        self.pc = address;
                        self.tick(ctx);
                    }
                }
                Condition::NC => {
                    if !self.registers.f.carry() {
                        self.pc = address;
                        self.tick(ctx);
                    }
                }
                Condition::C => {
                    if self.registers.f.carry() {
                        self.pc = address;
                        self.tick(ctx);
                    }
                }
            },
            _ => unimplemented!("Invalid target for operation"),
        }
    }

    pub(crate) fn jump_relative(&mut self, condition: Condition, offset: i8, ctx: &mut Context) {
        let condition_met = match condition {
            Condition::None => true,
            Condition::NZ => !self.registers.f.zero(),
            Condition::Z => self.registers.f.zero(),
            Condition::NC => !self.registers.f.carry(),
            Condition::C => self.registers.f.carry(),
        };
        if condition_met {
            self.tick(ctx);
            let result = self.pc.wrapping_add_signed(offset as i16);
            self.tick(ctx);
            self.pc = result;
        }
        self.tick(ctx);
    }

    pub(crate) fn rotate_accumulator(
        &mut self,
        rotation_type: RotationType,
        direction: Direction,
        _ctx: &mut Context,
    ) {
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
                    (self.registers.a << 1) | self.registers.f.carry().conv::<u8>(),
                    b7,
                )
            }
            (RotationType::NonCircular, Direction::Right) => {
                let b0 = self.registers.a & 0b1 == 1;
                (
                    (self.registers.a >> 1) | (self.registers.f.carry().conv::<u8>() << 7),
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

    pub(crate) fn rotate(
        &mut self,
        rotation_type: RotationType,
        direction: Direction,
        target: Target,
        ctx: &mut Context,
    ) {
        let prev_value = match target {
            Target::R8(register) => self.read_register8(register),
            Target::Ind(Indirect::R16(Registers16::HL)) => {
                let value = ctx.memory.read_u8(self.registers.hl());
                self.tick(ctx);
                value
            }
            _ => unimplemented!("Invalid target for operation"),
        };
        let (result, carry) = match (rotation_type, direction) {
            (RotationType::Circular, Direction::Left) => {
                let b7 = (prev_value >> 7) & 0b1 == 1;
                (prev_value.rotate_left(1), b7)
            }
            (RotationType::Circular, Direction::Right) => {
                let b0 = prev_value & 0b1 == 1;
                (prev_value.rotate_right(1), b0)
            }
            (RotationType::NonCircular, Direction::Left) => {
                let b7 = (prev_value >> 7) & 0b1 == 1;
                (
                    (prev_value << 1) | self.registers.f.carry().conv::<u8>(),
                    b7,
                )
            }
            (RotationType::NonCircular, Direction::Right) => {
                let b0 = prev_value & 0b1 == 1;
                (
                    (prev_value >> 1) | (self.registers.f.carry().conv::<u8>() << 7),
                    b0,
                )
            }
        };
        self.registers
            .f
            .set_zero(result == 0)
            .set_subtract(false)
            .set_half_carry(false)
            .set_carry(carry);
        match target {
            Target::R8(register) => {
                self.set_register8(register, result);
            }
            Target::Ind(Indirect::R16(Registers16::HL)) => {
                ctx.memory.set_u8(self.registers.hl(), result);
                self.tick(ctx);
            }
            _ => unimplemented!("Invalid target for operation"),
        }
    }
}

pub(crate) fn decompose_octal_triplet(value: u8) -> (u8, u8, u8) {
    ((value >> 6) & 0o7, (value >> 3) & 0o7, value & 0o7)
}

pub(crate) fn decompose_octal_cb(value: u8) -> (u8, u8) {
    ((value >> 3) & 0o77, value & 0o7)
}
