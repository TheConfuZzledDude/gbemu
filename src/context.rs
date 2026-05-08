use better_default::Default;

use crate::ppu::{LCDRegisters, Oam, Vram};

pub(crate) type Memory16K = [u8; 1024 * 16];

pub(crate) type Memory8K = [u8; 1024 * 8];

pub(crate) type Memory4K = [u8; 1024 * 4];

#[derive(Default)]
pub(crate) struct IoRegisters {
    pub(crate) joypad: JoypadRegister,
    pub(crate) serial: SerialTransferRegisters,
    pub(crate) timer: TimerRegisters,
    pub(crate) interrupt: Interrupts,
    pub(crate) lcd: LCDRegisters,
}

#[derive(Default)]
pub(crate) struct JoypadRegister;

impl JoypadRegister {
    pub(crate) fn read(&self) -> u8 {
        todo!()
    }

    pub(crate) fn write(&mut self, _value: u8) {
        todo!()
    }
}

#[derive(Default)]
pub(crate) struct SerialTransferRegisters;

impl SerialTransferRegisters {
    pub(crate) fn read(&self, _address: u8) -> u8 {
        todo!()
    }

    pub(crate) fn write(&mut self, _address: u8, _value: u8) {
        todo!()
    }
}

#[derive(Default)]
pub(crate) struct TimerRegisters {
    pub(crate) sc: u16,
    pub(crate) tima: u8,
    pub(crate) tma: u8,
    pub(crate) tac: u8,
    pub(crate) tima_overflow: Option<u8>,
    pub(crate) tima_written: bool,
}

impl TimerRegisters {
    pub(crate) fn read(&self, address: u8) -> u8 {
        match address {
            0x00..=0x03 => unreachable!(),
            0x04 => (self.sc >> 6) as u8,
            0x05 => self.tima,
            0x06 => self.tma,
            0x07 => self.tac,
            0x08.. => unreachable!(),
        }
    }

    pub(crate) fn write(&mut self, address: u8, value: u8) {
        match address {
            0x00..=0x03 => unreachable!(),
            0x04 => {
                let tac_enable = self.tac >> 2 & 0b1 == 0b1;
                let selected_bit = match self.tac & 0b11 {
                    0b00 => 8,
                    0b01 => 2,
                    0b10 => 4,
                    0b11 => 6,
                    _ => unreachable!(),
                };
                let sc_bit_prev = self.sc >> (selected_bit - 1) & 0b1 == 1;
                self.sc = 0;
                if sc_bit_prev && tac_enable {
                    self.timer_tick();
                }
            }
            0x05 => {
                self.tima_written = true;
                self.tima = value;
            }
            0x06 => {
                self.tma = value;
            }
            0x07 => {
                let tac_enable = self.tac >> 2 & 0b1 == 0b1;
                let selected_bit = match self.tac & 0b11 {
                    0b00 => 8,
                    0b01 => 2,
                    0b10 => 4,
                    0b11 => 6,
                    _ => unreachable!(),
                };
                let sc_bit_prev = self.sc >> (selected_bit - 1) & 0b1 == 1;
                self.tac = value;
                let selected_bit = match self.tac & 0b11 {
                    0b00 => 8,
                    0b01 => 2,
                    0b10 => 4,
                    0b11 => 6,
                    _ => unreachable!(),
                };
                let sc_bit_after = self.sc >> (selected_bit - 1) & 0b1 == 1;
                if sc_bit_prev && !sc_bit_after && tac_enable {
                    self.timer_tick();
                }
            }
            0x08.. => unreachable!(),
        };
    }

    pub(crate) fn timer_tick(&mut self) {
        let (result, overflow) = self.tima.overflowing_add(1);
        self.tima = result;
        if overflow && !self.tima_written {
            self.tima_overflow = Some(self.tma);
        }
    }

    pub(crate) fn clock_tick(&mut self) {
        let tac_enable = self.tac >> 2 & 0b1 == 0b1;
        let selected_bit = match self.tac & 0b11 {
            0b00 => 8,
            0b01 => 2,
            0b10 => 4,
            0b11 => 6,
            _ => unreachable!(),
        };
        let sc_bit_prev = self.sc >> (selected_bit - 1) & 0b1 == 1;
        self.sc = self.sc.wrapping_add(1);
        let sc_bit_new = self.sc >> (selected_bit - 1) & 0b1 == 1;
        if sc_bit_prev && !sc_bit_new && tac_enable {
            self.timer_tick();
        }
    }

    pub(crate) fn handle_overflow(&mut self, tma: u8, interrupts: &mut Interrupts) {
        self.tima = tma;

        // TODO: Schedule interrupt here
        interrupts.schedule_interrupt(InterruptType::Timer);
    }
}

#[derive(Default)]
pub(crate) struct Interrupts {
    pub(crate) interrupt_flag: u8,
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub(crate) enum InterruptType {
    Joypad = 4,
    Serial = 3,
    Timer = 2,
    LCD = 1,
    VBlank = 0,
}

impl Interrupts {
    pub(crate) fn read(&self) -> u8 {
        self.interrupt_flag
    }
    pub(crate) fn write(&mut self, value: u8) {
        self.interrupt_flag = value;
    }

    pub(crate) fn schedule_interrupt(&mut self, interrupt: InterruptType) {
        self.interrupt_flag |= 1 << (interrupt as u8);
    }

    pub(crate) fn clear_interrupt(&mut self, interrupt: InterruptType) {
        self.interrupt_flag &= !(1 << (interrupt as u8));
    }
}

impl IoRegisters {
    pub(crate) fn read_u8(&self, address: u8) -> u8 {
        match address {
            0x00 => self.joypad.read(),
            0x01..=0x02 => self.serial.read(address),
            0x03 => unimplemented!(),
            0x04..=0x07 => self.timer.read(address),
            0x08..0x0F => unimplemented!(),
            0x0F => self.interrupt.read(),
            0x10..=0x26 => {
                //TODO: AUDIO
                0xFF
            }
            0x27..0x30 => unimplemented!(),
            0x30..=0x3F => {
                //TODO: WAVE PATTERN
                0xFF
            }
            0x40..=0x4B => {
                //TODO: LCD/OAM
                0x90
            }

            0x50 => {
                // Bootrom bank control, write-only
                0xFF
            }
            0x80.. => unreachable!(),
            _ => unimplemented!("CGB/unused IO address"),
        }
    }

    pub(crate) fn write_u8(&mut self, address: u8, value: u8) {
        match address {
            0x00 => self.joypad.write(value),
            0x01..=0x02 => self.serial.write(address, value),
            0x03 => unimplemented!(),
            0x04..=0x07 => self.timer.write(address, value),
            0x08..0x0F => unimplemented!(),
            0x0F => self.interrupt.write(value),
            0x10..=0x26 => {
                //TODO: AUDIO
            }
            0x27..0x30 => unimplemented!(),
            0x30..=0x3F => {
                //TODO: WAVE PATTERN
            }
            0x40..=0x4B => {
                //TODO: LCD/OAM
            }

            0x50 => {
                // Bootrom bank control, write-only
            }
            0x80.. => unreachable!(),
            _ => unimplemented!("CGB/unused IO address"),
        }
    }
}

#[derive(Default)]
pub(crate) struct MemoryBus {
    #[default([0; 1024*16])]
    pub(crate) rom: Memory16K,
    #[default(vec![[0; 1024*16]])]
    pub(crate) rom_banks: Vec<Memory16K>,
    pub(crate) vram: Vram,
    #[default([0; 1024 * 8])]
    pub(crate) external_ram: Memory8K,
    #[default([0; 1024 * 4])]
    pub(crate) wram1: Memory4K,
    #[default([0; 1024 * 4])]
    pub(crate) wram2: Memory4K,
    pub(crate) oam: Oam,
    pub(crate) io: IoRegisters,
    #[default([0; 0xFFFF-0xFF80])]
    pub(crate) hram: [u8; 0xFFFF - 0xFF80],
    pub(crate) ie: u8,
}

impl MemoryBus {
    pub(crate) fn read_u8(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => self.rom[address as usize],
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
            0xFF00..=0xFF7F => self.io.read_u8(address as u8),
            0xFF80..=0xFFFE => self.hram[address as usize - 0xFF80],
            0xFFFF => self.ie,
        }
    }

    pub(crate) fn set_u8(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x3FFF => self.rom[address as usize] = value,
            0x4000..=0x7FFF => {
                // TODO: switchable rom banks
                self.rom_banks[0][address as usize - 0x4000] = value
            }
            0x8000..=0x9FFF => self.vram[address as usize - 0x8000] = value,
            0xA000..=0xBFFF => self.external_ram[address as usize - 0xA000] = value,
            0xC000..=0xCFFF => self.wram1[address as usize - 0xC000] = value,
            0xD000..=0xDFFF => self.wram2[address as usize - 0xD000] = value,
            0xE000..=0xFDFF => {
                //Echo RAM
                self.wram1[address as usize - 0xE000] = value
            }
            0xFE00..=0xFE9F => {
                todo!("Implement OAM")
            }
            0xFEA0..=0xFEFF => {
                todo!("Prohibited region, implement undefined behaviour")
            }
            0xFF00..=0xFF7F => {
                self.io.write_u8(address as u8, value);
            }
            0xFF80..=0xFFFE => self.hram[address as usize - 0xFF80] = value,
            0xFFFF => self.ie = value,
        }
    }
}

#[derive(Default)]
pub(crate) struct Context {
    pub(crate) memory: MemoryBus,
}
