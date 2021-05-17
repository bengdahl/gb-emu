use crate::cpu::{CpuInputPins, CpuOutputPins};

pub mod monochrome;
pub mod registers;

pub trait PPU {
    type Frame;

    fn clock_t_state(&mut self);
    fn perform_io(&mut self, input: CpuOutputPins) -> CpuInputPins;
    fn get_frame(&self) -> Self::Frame;
}

impl<T: PPU> super::Chip for T {
    fn chip_select(&self, addr: u16) -> bool {
        match addr {
            0x8000..=0x9FFF | 0xFE00..=0xFE9F | 0xFF40..=0xFF4B => true,
            _ => false,
        }
    }

    #[inline]
    fn clock_unselected(&mut self) {
        for _ in 0..4 {
            self.clock_t_state()
        }
    }

    #[inline]
    fn clock(&mut self, input: CpuOutputPins) -> CpuInputPins {
        let output = self.perform_io(input);

        self.clock_unselected();

        output
    }
}
