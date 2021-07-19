pub mod cart;
pub mod memory;
pub mod ppu;
pub mod timer;

use crate::cpu::{CpuInputPins, CpuOutputPins, CpuRunner};
use memory::Memory;
use ppu::PPU;

use self::{cart::Cart, models::DMG};

pub struct Gameboy<Model: models::GbModel> {
    pub cpu: CpuRunner,
    pub ppu: Model::PPU,
    cpu_input: CpuInputPins,
    memory: Memory,
    pub cart: cart::Cart,
    timer: timer::Timer,

    interrupt_enable: u8,
    interrupt_request: u8,
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
            timer: timer::Timer::default(),

            interrupt_enable: 0,
            interrupt_request: 0,
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
        // static mut BREAKPOINT_INSTR: bool = false;
        // static mut BREAKPOINT_E8: bool = false;
        // unsafe {
        //     if let CpuOutputPins::Read { addr: 0xc60a } = cpu_out {
        //         BREAKPOINT_E8 = true;
        //     }
        //     if let CpuOutputPins::Read { addr: 0xdef8 } = cpu_out {
        //         if BREAKPOINT_E8 {
        //             BREAKPOINT_INSTR = true;
        //         }
        //     }
        //     if BREAKPOINT_INSTR && BREAKPOINT_E8 {
        //         println!("{:?}", self.cpu.cpu);
        //     }
        // }
        if let CpuOutputPins::Write { addr: 0xff01, data } = cpu_out {
            print!("{}", data as char)
        }

        let chips: &mut [&mut dyn Chip] = &mut [
            &mut self.ppu,
            &mut self.memory,
            &mut self.cart,
            &mut self.timer,
        ];

        let bus_output = {
            let mut data = 0xFF;
            let mut ir = self.interrupt_request;

            for chip in chips {
                chip.clock(cpu_out, &mut data, &mut ir);
            }

            self.interrupt_request = ir;
            data
        };

        // Handle changes to IE & IF (handled independently from chips)
        match cpu_out {
            CpuOutputPins::Write { addr: 0xFF0F, data } => self.interrupt_request = data & 0x1F,
            CpuOutputPins::Write { addr: 0xFFFF, data } => self.interrupt_enable = data & 0x1F,
            _ => (),
        };

        let interrupt_requests = self.interrupt_enable & self.interrupt_request;
        self.cpu_input = CpuInputPins {
            interrupt_40h: interrupt_requests & (1 << 0) != 0,
            interrupt_48h: interrupt_requests & (1 << 1) != 0,
            interrupt_50h: interrupt_requests & (1 << 2) != 0,
            interrupt_58h: interrupt_requests & (1 << 3) != 0,
            interrupt_60h: interrupt_requests & (1 << 4) != 0,

            // IE & IF are not part of any chip, so they must be handled separately
            data: match cpu_out {
                CpuOutputPins::Read { addr: 0xFF0F } => self.interrupt_request,
                CpuOutputPins::Read { addr: 0xFFFF } => self.interrupt_enable,
                _ => bus_output,
            },
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

/// Using this trait makes it easy to clock every chip on the Gameboy independently
trait Chip {
    /// Clock by one M-cycle
    fn clock(&mut self, input: CpuOutputPins, data: &mut u8, interrupt_request: &mut u8);
}
