use gb_cpu::CpuOutputPins;

pub struct Memory {
    work_ram_1: [u8; 0x1000],
    work_ram_2: [u8; 0x1000],
    high_ram: [u8; 0x7f],
}

impl Memory {
    pub fn new() -> Self {
        Memory {
            work_ram_1: [0; 0x1000],
            work_ram_2: [0; 0x1000],
            high_ram: [0; 0x7f],
        }
    }

    fn address_is_in_range(addr: u16) -> bool {
        matches!(addr, 0xC000..=0xDFFF | 0xFF80..=0xFFFE)
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Index<u16> for Memory {
    type Output = u8;
    fn index(&self, index: u16) -> &Self::Output {
        match index {
            0xC000..=0xCFFF => &self.work_ram_1[(index - 0xC000) as usize],
            0xD000..=0xDFFF => &self.work_ram_2[(index - 0xD000) as usize],
            0xFF80..=0xFFFE => &self.high_ram[(index - 0xFF80) as usize],
            _ => panic!("Out of bounds: {}", index),
        }
    }
}

impl std::ops::IndexMut<u16> for Memory {
    fn index_mut(&mut self, index: u16) -> &mut Self::Output {
        match index {
            0xC000..=0xCFFF => &mut self.work_ram_1[(index - 0xC000) as usize],
            0xD000..=0xDFFF => &mut self.work_ram_2[(index - 0xD000) as usize],
            0xFF80..=0xFFFE => &mut self.high_ram[(index - 0xFF80) as usize],
            _ => panic!("Out of bounds: {}", index),
        }
    }
}

impl super::Chip for Memory {
    fn clock(&mut self, input: CpuOutputPins, data: &mut u8, _interrupt_request: &mut u8) {
        if Self::address_is_in_range(input.addr()) {
            match input {
                CpuOutputPins::Read { addr } => {
                    *data = self[addr];
                }
                CpuOutputPins::Write { addr, data } => {
                    self[addr] = data;
                }
            }
        }
    }
}
