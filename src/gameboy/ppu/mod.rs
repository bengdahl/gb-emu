pub mod monochrome;
pub mod registers;

#[derive(Debug, Clone, Copy)]
pub enum PpuInputPins {
    Read { addr: u16 },
    Write { addr: u16, data: u8 },
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PpuOutputPins {
    pub data: u8,

    pub vblank_interrupt: bool,
    pub stat_interrupt: bool,
}

pub trait PPU {
    type Frame;

    fn clock(&mut self, input: Option<PpuInputPins>) -> PpuOutputPins;
    fn get_frame(&self) -> Self::Frame;
}
