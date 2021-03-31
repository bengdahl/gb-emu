use crate::cpu::CpuRunner;

pub struct Gameboy<Model> {
    cpu: CpuRunner,
    _model: std::marker::PhantomData<Model>,
}

pub mod models {
    pub trait GbModel {}

    /// The original Gameboy
    pub enum DMG {}
    impl GbModel for DMG {}
    /// The Gameboy Color
    pub enum GBC {}
    impl GbModel for GBC {}
    /// The Super Gameboy SNES Cartridge
    pub enum SGB {}
    impl GbModel for SGB {}
}

impl<Model: models::GbModel> Gameboy<Model> {
    /// Clock the entire gameboy by one T-state
    pub fn clock(&mut self) {}
}
