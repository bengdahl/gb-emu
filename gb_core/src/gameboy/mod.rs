pub mod cart;
pub mod joypad;
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
    pub memory: Memory,
    pub cart: cart::Cart,
    timer: timer::Timer,
    pub joypad: joypad::Joypad,

    cpu_input: CpuInputPins,
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
            joypad: joypad::Joypad::default(),

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

        let chips: &mut [&mut dyn Chip] = &mut [
            &mut self.ppu,
            &mut self.memory,
            &mut self.cart,
            &mut self.timer,
            &mut self.joypad,
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
    /// Fetches a frame from the PPU, scales it, and returns it with its wdth and height
    pub fn get_frame(&self, scale: impl Into<Option<usize>>) -> (Vec<u32>, usize, usize) {
        let scale = scale.into().unwrap_or(1);
        let frame = self.ppu.get_frame();
        let width = frame.width * scale;
        let height = frame.height * scale;
        let frame = {
            let mut new_frame = vec![0; width * height];

            frame
                .pixels
                .chunks_exact(frame.width)
                .enumerate()
                .for_each(|(y, row)| {
                    let row_offset = y * scale;
                    for yoff in row_offset..row_offset + scale {
                        for (x, pixel) in row.iter().enumerate() {
                            let pix_offset = x * scale;
                            for xoff in pix_offset..pix_offset + scale {
                                new_frame[yoff * width + xoff] = *pixel;
                            }
                        }
                    }
                });

            new_frame
        };

        (frame, width, height)
    }
}

/// Using this trait makes it easy to clock every chip on the Gameboy independently
trait Chip {
    /// Clock by one M-cycle
    fn clock(&mut self, input: CpuOutputPins, data: &mut u8, interrupt_request: &mut u8);
}
