pub mod cart;
pub mod memory;
pub mod ppu;

use crate::cpu::{CpuInputPins, CpuOutputPins, CpuRunner};
use memory::Memory;
use ppu::PPU;

use self::{cart::Cart, models::DMG};

pub struct Gameboy<Model: models::GbModel> {
    cpu: CpuRunner,
    ppu: Model::PPU,
    cpu_input: CpuInputPins,
    memory: Memory,
    cart: cart::Cart,
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

impl Gameboy<DMG> {
    pub fn new(rom: Vec<u8>) -> Result<Self, &'static str> {
        Ok(Gameboy {
            cpu: crate::cpu::Cpu::default().runner(),
            ppu: ppu::monochrome::MonochromePpu::new(),
            cpu_input: CpuInputPins::default(),
            memory: Memory::new(),
            cart: Cart::new(rom)?,
        })
    }

    /// temporary
    pub fn reset(&mut self) {
        self.cpu.cpu.registers.pc = 0x100;
        self.cpu.cpu.registers.sp = 0xFFFE;
    }
}

impl<Model: models::GbModel> Gameboy<Model> {
    /// Clock the entire gameboy by M-cycle
    pub fn clock(&mut self) {
        let cpu_out = self.cpu.clock(self.cpu_input);

        // remove later
        static mut BREAKPOINT_HIT: bool = false;
        let addr = match cpu_out {
            CpuOutputPins::Write { addr, .. } => addr,
            CpuOutputPins::Read { addr } => addr,
        };
        unsafe {
            if addr == 0x63b {
                BREAKPOINT_HIT = true;
            }
            if BREAKPOINT_HIT {
                println!("{:?}", self.cpu.cpu);
            }
        }
        if let CpuOutputPins::Write { addr: 0xff01, data } = cpu_out {
            print!("{}", data as char)
        }

        let chips: &mut [&mut dyn Chip] = &mut [&mut self.ppu, &mut self.memory, &mut self.cart];

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
                    // println!("empty bus: {:X?}", cpu_out);
                    CpuInputPins::default()
                }
            }
        }
    }
}
impl Gameboy<DMG> {
    pub fn get_frame(&self) -> Vec<u8> {
        let frame = self.ppu.get_frame();
        frame
            .pixels
            .iter()
            .copied()
            .flat_map(|p| p.to_le_bytes())
            .collect()
    }
}

pub trait Chip {
    fn chip_select(&self, addr: u16) -> bool;
    /// Clock by one M-cycle
    fn clock(&mut self, input: CpuOutputPins) -> CpuInputPins;
    /// Clock by one M-cycle with the chip unselected (addr is not in this chip's range)
    fn clock_unselected(&mut self);
}
