use crate::cpu::CpuOutputPins;

use super::Chip;

#[derive(Default, Debug)]
pub struct Timer {
    div: u16,
    tima: u8,
    tma: u8,
    tac: u8,
}

impl Chip for Timer {
    fn clock(
        &mut self,
        input: crate::cpu::CpuOutputPins,
        data: &mut u8,
        interrupt_request: &mut u8,
    ) {
        let mut tima_write = false;

        match input {
            // DIV
            CpuOutputPins::Write { addr: 0xFF04, .. } => self.div = 0,
            CpuOutputPins::Read { addr: 0xFF04 } => *data = (self.div >> 8) as u8,

            // TIMA
            CpuOutputPins::Write {
                addr: 0xFF05,
                data: v,
            } => {
                self.tima = v;
                tima_write = true;
            }
            CpuOutputPins::Read { addr: 0xFF05 } => *data = self.tima,

            // TMA
            CpuOutputPins::Write {
                addr: 0xFF06,
                data: v,
            } => self.tma = v,
            CpuOutputPins::Read { addr: 0xFF06 } => *data = self.tma,

            // TAC
            CpuOutputPins::Write {
                addr: 0xFF07,
                data: v,
            } => self.tac = v,
            CpuOutputPins::Read { addr: 0xFF07 } => *data = self.tac,
            _ => (),
        };

        self.div = self.div.wrapping_add(4);

        let div_compare = match self.tac & 0b11 {
            0b00 => 1024,
            0b01 => 16,
            0b10 => 64,
            0b11 => 256,
            _ => unreachable!(),
        };

        let timer_enable = self.tac & 0b100 != 0;

        let timer_inc = timer_enable && self.div % div_compare == 0;

        if !tima_write && timer_inc {
            let (tima, carry) = self.tima.overflowing_add(1);
            self.tima = tima;
            if carry {
                // Set interrupt 50h
                *interrupt_request = *interrupt_request | 0b100;
                // Reset TIMA to TMA
                self.tima = self.tma;
            }
        }
    }
}
