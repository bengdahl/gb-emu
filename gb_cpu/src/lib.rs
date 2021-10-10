#![feature(generators, generator_trait, never_type, destructuring_assignment)]

pub mod assembler;
mod decode;
mod execute;
mod registers;

pub use execute::{CpuRunner, CpuRunnerYield};
pub use registers::{FRegister, Registers};

/// Contains the state of a LR35902 CPU.
#[derive(Clone, Copy, Debug, Default)]
pub struct Cpu {
    pub registers: Registers,
    pub ime: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum CpuOutputPins {
    Read { addr: u16 },
    Write { addr: u16, data: u8 },
}

impl CpuOutputPins {
    #[inline]
    pub fn addr(&self) -> u16 {
        match self {
            Self::Read { addr } => *addr,
            Self::Write { addr, .. } => *addr,
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct CpuInputPins {
    pub data: u8,
    pub interrupt_40h: bool,
    pub interrupt_48h: bool,
    pub interrupt_50h: bool,
    pub interrupt_58h: bool,
    pub interrupt_60h: bool,
}
