pub mod simple;

#[derive(Clone)]
pub struct Frame {
    pub pixels: Vec<u32>,
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct PpuInputPins {
    pub addr: u16,
    pub data: u8,
    pub is_read: bool,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PpuOutputPins {
    pub data: u8,

    pub vblank_interrupt: bool,
    pub stat_interrupt: bool,
}

pub trait PPU {
    fn clock(&mut self, input: PpuInputPins) -> PpuOutputPins;
    fn get_frame(&self) -> &Frame;
}
