pub mod memory;
pub mod ppu;

use crate::cpu::{CpuInputPins, CpuOutputPins, CpuRunner};
use memory::Memory;
use ppu::PPU;

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
    /// Clock the entire gameboy by M-cycle
    pub fn clock(&mut self) {
        let cpu_out = self.cpu.clock(self.cpu_input);

        let chips: &mut [&mut dyn Chip] = &mut [&mut self.ppu as &mut dyn Chip, &mut self.memory];

        self.cpu_input = {
            let mut chip_outputs = chips.iter_mut().filter_map(|chip| {
                if chip.chip_select(cpu_out.addr()) {
                    Some(chip.clock(cpu_out))
                } else {
                    chip.clock_unselected();
                    None
                }
            });
            let cpu_input = chip_outputs.next();
            if !chip_outputs.next().is_none() {
                println!("bus conflict: {:?}", self.cpu_input);
            }
            match cpu_input {
                Some(c) => c,
                None => {
                    println!("empty bus: {:?}", self.cpu_input);
                    CpuInputPins::default()
                }
            }
        }

        // let cpu_input = match cpu_out {
        //     CpuOutputPins::Read { addr } => match addr {
        //         0x0000..=0x7FFF => todo!("Cartridge ROM support"),
        //         0x8000..=0x9FFF | 0xFE00..=0xFE9F => self.ppu.clock(cpu_out),
        //         0xA000..=0xBFFF => todo!("Cartridge RAM support"),
        //         0xC000..=0xDFFF | 0xFF80..=0xFFFE => {
        //             let v = self.memory[addr];
        //             cpu_input = CpuInputPins {
        //                 data: v,
        //                 ..Default::default()
        //             };
        //         }
        //         0xE000..=0xFDFF => todo!("Echo address support"),
        //         0xFEA0..=0xFF7F => todo!("IO"),
        //         0xFFFF => todo!("IE"),
        //     },
        //     CpuOutputPins::Write { addr, data } => match addr {
        //         0x0000..=0x7FFF => todo!("Cartridge ROM support"),
        //         0x8000..=0x9FFF | 0xFE00..=0xFE9F => ppu_input = PpuInputPins::Write { addr, data },
        //         0xA000..=0xBFFF => todo!("Cartridge RAM support"),
        //         0xC000..=0xDFFF | 0xFF80..=0xFFFE => self.memory[addr] = data,
        //         0xE000..=0xFDFF => todo!("Echo address support"),
        //         0xFEA0..=0xFF7F => todo!("IO"),
        //         0xFFFF => todo!("IE"),
        //     },
        // }
    }
}

pub trait Chip {
    fn chip_select(&self, addr: u16) -> bool;
    /// Clock by one M-cycle
    fn clock(&mut self, input: CpuOutputPins) -> CpuInputPins;
    /// Clock by one M-cycle with the chip unselected (addr is not in this chip's range)
    fn clock_unselected(&mut self);
}
