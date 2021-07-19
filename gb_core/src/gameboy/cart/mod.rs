mod mbc1;
mod rom;

use super::Chip;
use crate::cpu::CpuOutputPins;
use mbc1::{Mbc1, Mbc1WithBatteryRam, Mbc1WithRam};

trait Mapper: Chip {}

pub struct Cart {
    mapper: Box<dyn Mapper + Send>,
}

impl Chip for Cart {
    fn clock(&mut self, input: CpuOutputPins, data: &mut u8, interrupt_request: &mut u8) {
        self.mapper.clock(input, data, interrupt_request)
    }
}

impl Cart {
    pub fn new(data: Vec<u8>) -> Result<Self, &'static str> {
        let id = data.get(0x147).ok_or("Invalid ROM file")?;
        let mapper = mapper_from_id(*id, data);
        Ok(Cart { mapper })
    }
}

fn mapper_from_id(id: u8, data: Vec<u8>) -> Box<dyn Mapper + Send> {
    match id {
        0 => Box::new(rom::Rom::new(data)),
        1 => Box::new(Mbc1::new(data)),
        2 => Box::new(Mbc1WithRam::new(data)),
        3 => Box::new(Mbc1WithBatteryRam::new(data)),
        _ => panic!("Mapper unimplemented: {:#02X}", id),
    }
}
