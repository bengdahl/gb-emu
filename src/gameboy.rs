use crate::{
    cpu::{CpuInputPins, CpuRunner},
    ppu::{self, PpuInputPins},
};

pub struct Gameboy<Model> {
    cpu: CpuRunner,
    ppu: Box<dyn ppu::PPU>,
    cpu_input: CpuInputPins,

    work_ram_1: [u8; 0x1000],
    work_ram_2: [u8; 0x1000],
    high_ram: [u8; 0x7f],

    _model: std::marker::PhantomData<Model>,
}

pub mod models {
    pub trait GbModel {}

    /// The original Gameboy
    pub enum DMG {}
    impl GbModel for DMG {}
    /// The Gameboy Color
    pub enum GBC {}
    impl GbModel for GBC {}
    /// The Super Gameboy SNES Cartridge
    pub enum SGB {}
    impl GbModel for SGB {}
}

impl<Model: models::GbModel> Gameboy<Model> {
    /// Clock the entire gameboy by M-cycle
    pub fn clock(&mut self) {
        let cpu_out = self.cpu.clock(self.cpu_input);

        let cpu_input: CpuInputPins;
        let ppu_input: PpuInputPins;

        if cpu_out.is_read {
            match cpu_out.addr {
                0x0000..=0x7FFF => todo!("Cartridge ROM support"),
                0x8000..=0x9FFF | 0xFE00..=0xFE9F => {
                    ppu_input = PpuInputPins {
                        addr: cpu_out.addr,
                        data: cpu_out.data,
                        is_read: cpu_out.is_read,
                    }
                }
                0xA000..=0xBFFF => todo!("Cartridge RAM support"),
                0xC000..=0xCFFF => {
                    let v = self.work_ram_1[(cpu_out.addr - 0xC000) as usize];
                    cpu_input = CpuInputPins {
                        data: v,
                        ..Default::default()
                    };
                }
                0xD000..=0xDFFF => {
                    let v = self.work_ram_2[(cpu_out.addr - 0xD000) as usize];
                    cpu_input = CpuInputPins {
                        data: v,
                        ..Default::default()
                    };
                }
                0xE000..=0xFDFF => todo!("Echo address support"),
                0xFEA0..=0xFF7F => todo!("IO"),
                0xFF80..=0xFFFE => {
                    let v = self.high_ram[(cpu_out.addr - 0xFF80) as usize];
                    cpu_input = CpuInputPins {
                        data: v,
                        ..Default::default()
                    };
                }
                0xFFFF => todo!("IE")
            }
        } else {
            match cpu_out.addr {
                0x0000..=0x7FFF => todo!("Cartridge ROM support"),
                0x8000..=0x9FFF | 0xFE00..=0xFE9F => {
                    ppu_input = PpuInputPins {
                        addr: cpu_out.addr,
                        data: cpu_out.data,
                        is_read: cpu_out.is_read,
                    }
                }
                0xA000..=0xBFFF => todo!("Cartridge RAM support"),
                0xC000..=0xCFFF => self.work_ram_1[(cpu_out.addr - 0xC000) as usize] = cpu_out.data,
                0xD000..=0xDFFF => self.work_ram_2[(cpu_out.addr - 0xD000) as usize] = cpu_out.data,
                0xE000..=0xFDFF => todo!("Echo address support"),
                0xFEA0..=0xFF7F => todo!("IO"),
                0xFF80..=0xFFFE => self.high_ram[(cpu_out.addr - 0xFF80) as usize] = cpu_out.data,
                0xFFFF => todo!("IE")
            }
        }
    }
}
