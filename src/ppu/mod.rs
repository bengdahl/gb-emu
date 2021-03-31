pub mod simple;

#[derive(Clone)]
pub struct Frame {
    pub pixels: Vec<u32>,
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct PpuInputPins {
    addr: u16,
    data: u8,
    is_write: bool,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PpuOutputPins {
    vblank_interrupt: bool,
    stat_interrupt: bool,
}

pub trait PPU {
    fn clock(&mut self, input: PpuInputPins) -> PpuOutputPins;
    fn get_frame(&self) -> &Frame;
}
