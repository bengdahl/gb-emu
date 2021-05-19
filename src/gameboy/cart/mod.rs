mod rom;

use super::Chip;
use crate::cpu::{CpuInputPins, CpuOutputPins};

trait Mapper: Chip {}

pub struct Cart {
    mapper: Box<dyn Mapper>,
}

impl Chip for Cart {
    fn clock(&mut self, input: CpuOutputPins) -> CpuInputPins {
        self.mapper.clock(input)
    }

    fn clock_unselected(&mut self) {
        self.mapper.clock_unselected()
    }

    fn chip_select(&self, addr: u16) -> bool {
        self.mapper.chip_select(addr)
    }
}

impl Cart {
    pub fn new(data: Vec<u8>) -> Result<Self, &'static str> {
        let id = data.get(0x147).ok_or("Invalid ROM file")?;
        let mapper = mapper_from_id(*id, data);
        Ok(Cart { mapper })
    }
}

fn mapper_from_id(id: u8, data: Vec<u8>) -> Box<dyn Mapper> {
    match id {
        0 => Box::new(rom::Rom::new(data)),
        _ => panic!("Mapper unimplemented: {:02X}", id),
    }
}
