use crate::cpu::CpuOutputPins;

pub mod monochrome;
pub mod registers;

pub trait PPU {
    type Frame;

    fn clock_t_state(&mut self);
    fn perform_io(&mut self, input: CpuOutputPins, data: &mut u8, interrupt_request: &mut u8);
    fn get_frame(&self) -> Self::Frame;
}

impl<T: PPU> super::Chip for T {
    #[inline]
    fn clock(&mut self, input: CpuOutputPins, data: &mut u8, interrupt_request: &mut u8) {
        self.perform_io(input, data, interrupt_request);

        for _ in 0..4 {
            self.clock_t_state()
        }
    }
}
