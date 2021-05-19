use super::*;

pub struct Rom {
    pub data: [u8; 0x8000],
}

impl Rom {
    pub fn new(data: Vec<u8>) -> Self {
        let mut buf = [0; 0x8000];
        let len = usize::max(data.len(), 0x8000);
        buf[..len].copy_from_slice(&data[..len]);
        Self { data: buf }
    }
}

impl Chip for Rom {
    fn clock_unselected(&mut self) {}
    fn clock(&mut self, input: CpuOutputPins) -> CpuInputPins {
        match input {
            CpuOutputPins::Read { addr } => CpuInputPins {
                data: self.data[addr as usize],
                ..Default::default()
            },
            CpuOutputPins::Write { .. } => CpuInputPins::default(),
        }
    }
    fn chip_select(&self, addr: u16) -> bool {
        (0x0000..=0x7FFF).contains(&addr)
    }
}
impl Mapper for Rom {}
