pub mod cart;
pub mod joypad;
pub mod memory;
pub mod ppu;
pub mod timer;

use crate::cpu::{CpuInputPins, CpuOutputPins, CpuRunner, CpuRunnerYield};
use memory::Memory;

use self::{cart::Cart, ppu::Ppu};

pub struct Gameboy {
    pub cpu: CpuRunner,
    pub ppu: Ppu,
    pub memory: Memory,
    pub cart: cart::Cart,
    timer: timer::Timer,
    pub joypad: joypad::Joypad,

    cpu_input: CpuInputPins,
    interrupt_enable: u8,
    interrupt_request: u8,
}

impl Gameboy {
    pub fn new(rom: Vec<u8>) -> Result<Self, &'static str> {
        Ok(Gameboy {
            cpu: crate::cpu::Cpu::default().runner(),
            ppu: ppu::Ppu::new(),
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

/// Contains information about a clock cycle for use by debugging methods
pub struct ClockDebug {
    pub is_fetch_cycle: bool,
    pub opcode_fetched: Option<u16>,
}

impl Gameboy {
    /// Clock the entire gameboy by M-cycle
    pub fn clock(&mut self) -> ClockDebug {
        let CpuRunnerYield {
            pins: cpu_pins_out,
            is_fetch_cycle,
        } = self.cpu.clock(self.cpu_input);

        let opcode_fetched = if is_fetch_cycle {
            Some(cpu_pins_out.addr())
        } else {
            None
        };

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
                chip.clock(cpu_pins_out, &mut data, &mut ir);
            }

            self.interrupt_request = ir;
            data
        };

        // Handle changes to IE & IF (handled independently from chips)
        match cpu_pins_out {
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
            data: match cpu_pins_out {
                CpuOutputPins::Read { addr: 0xFF0F } => self.interrupt_request,
                CpuOutputPins::Read { addr: 0xFFFF } => self.interrupt_enable,
                _ => bus_output,
            },
        };

        ClockDebug {
            is_fetch_cycle,
            opcode_fetched,
        }
    }

    /// Clock the gameboy by the time it takes to complete one instruction
    pub fn step_instruction(&mut self) {
        loop {
            if let ClockDebug {
                is_fetch_cycle: true,
                ..
            } = self.clock()
            {
                break;
            }
        }
    }
}
impl Gameboy {
    /// Fetches a frame from the PPU
    pub fn get_frame(&self) -> Box<ppu::frame::Frame> {
        self.ppu.get_frame()
    }
}

/// Using this trait makes it easy to clock every chip on the Gameboy independently
trait Chip {
    /// Clock by one M-cycle
    fn clock(&mut self, input: CpuOutputPins, data: &mut u8, interrupt_request: &mut u8);
}
