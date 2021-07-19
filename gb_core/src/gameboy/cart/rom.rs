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
    fn clock(&mut self, input: CpuOutputPins, data: &mut u8, _interrupt_request: &mut u8) {
        if let CpuOutputPins::Read {
            addr: addr @ (0x0000..=0x7FFF),
        } = input
        {
            *data = self.data[addr as usize]
        }
    }
}
impl Mapper for Rom {}
