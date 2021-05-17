pub mod memory;
pub mod ppu;

use crate::cpu::{CpuInputPins, CpuOutputPins, CpuRunner};
use memory::Memory;
use ppu::{PpuInputPins, PPU};

pub struct Gameboy<Model: models::GbModel> {
    cpu: CpuRunner,
    ppu: Model::PPU,
    cpu_input: CpuInputPins,
    memory: Memory,
}

pub mod models {
    use super::*;
    pub trait GbModel {
        type PPU: PPU;
    }

    /// The original Gameboy
    pub enum DMG {}
    impl GbModel for DMG {
        type PPU = ppu::monochrome::MonochromePpu;
    }
    // /// The Gameboy Color
    // pub enum GBC {}
    // impl GbModel for GBC {}
    // /// The Super Gameboy SNES Cartridge
    // pub enum SGB {}
    // impl GbModel for SGB {}
}

impl<Model: models::GbModel> Gameboy<Model> {
    /// Clock all chips on the device (except the CPU) by one M-cycle (1 CPU/memory cycle, 4 PPU cycles)
    fn clock_all_chips(&mut self, cpu_out: CpuOutputPins) {
        enum ChipSelect {
            Memory,
            Ppu,
        }
    }

    /// Clock the entire gameboy by M-cycle
    pub fn clock(&mut self) {
        let cpu_out = self.cpu.clock(self.cpu_input);

        let cpu_input: CpuInputPins;
        let ppu_input: PpuInputPins;

        if cpu_out.is_read {
            match cpu_out.addr {
                0x0000..=0x7FFF => todo!("Cartridge ROM support"),
                0x8000..=0x9FFF | 0xFE00..=0xFE9F => {
                    ppu_input = PpuInputPins::Read { addr: cpu_out.addr }
                }
                0xA000..=0xBFFF => todo!("Cartridge RAM support"),
                0xC000..=0xDFFF | 0xFF80..=0xFFFE => {
                    let v = self.memory[cpu_out.addr];
                    cpu_input = CpuInputPins {
                        data: v,
                        ..Default::default()
                    };
                }
                0xE000..=0xFDFF => todo!("Echo address support"),
                0xFEA0..=0xFF7F => todo!("IO"),
                0xFFFF => todo!("IE"),
            }
        } else {
            match cpu_out.addr {
                0x0000..=0x7FFF => todo!("Cartridge ROM support"),
                0x8000..=0x9FFF | 0xFE00..=0xFE9F => {
                    ppu_input = PpuInputPins::Write {
                        addr: cpu_out.addr,
                        data: cpu_out.data,
                    }
                }
                0xA000..=0xBFFF => todo!("Cartridge RAM support"),
                0xC000..=0xDFFF | 0xFF80..=0xFFFE => self.memory[cpu_out.addr] = cpu_out.data,
                0xE000..=0xFDFF => todo!("Echo address support"),
                0xFEA0..=0xFF7F => todo!("IO"),
                0xFFFF => todo!("IE"),
            }
        }
    }
}
