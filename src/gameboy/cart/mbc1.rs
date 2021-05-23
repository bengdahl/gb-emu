use crate::{
    cpu::{CpuInputPins, CpuOutputPins},
    gameboy::Chip,
};

use super::Mapper;

type Bank = [u8; 0x4000];

pub type Mbc1 = Mbc1Generic<ram::NullRam>;
pub type Mbc1WithRam = Mbc1Generic<ram::BasicRam>;
// TODO: Implement save files
pub type Mbc1WithBatteryRam = Mbc1Generic<ram::BasicRam>;

// TODO: ROM Bank mirroring
pub struct Mbc1Generic<R: ram::Ram> {
    data: Vec<Bank>,
    ram: R,

    ram_enable: bool,
    rom_bank_lower: u8,
    rom_bank_upper: u8,
    mode_select: bool,
}

impl<R: ram::Ram> Mbc1Generic<R> {
    pub fn new(data: Vec<u8>) -> Self {
        let mut banks = data.array_chunks::<0x4000>();
        let mut data = vec![];
        while let Some(bank) = banks.next() {
            data.push(bank.clone());
        }
        let remainder = {
            let mut buf = [0; 0x4000];
            buf[..banks.remainder().len()].copy_from_slice(banks.remainder());
            buf
        };

        data.push(remainder);

        while data.len() < 0x80 {
            data.push([0; 0x4000]);
        }

        assert_eq!(data.len(), 0x80);

        Mbc1Generic {
            data,
            ram: Default::default(),
            ram_enable: false,
            rom_bank_lower: 1,
            rom_bank_upper: 0,
            mode_select: false,
        }
    }

    fn bank_0(&mut self) -> &mut [u8; 0x4000] {
        let bank_idx = if self.mode_select {
            self.rom_bank_upper << 5
        } else {
            0
        };
        &mut self.data[bank_idx as usize]
    }

    fn bank_1(&mut self) -> &mut [u8; 0x4000] {
        let lower = if self.rom_bank_lower == 0 {
            1
        } else {
            self.rom_bank_lower
        };
        let bank_idx = if self.mode_select {
            self.rom_bank_upper << 5 + lower
        } else {
            0
        };
        &mut self.data[bank_idx as usize]
    }
}

impl<R: ram::Ram> Chip for Mbc1Generic<R> {
    fn chip_select(&self, addr: u16) -> bool {
        matches!(addr, 0x0000..=0x7FFF | 0xA000..=0xBFFF)
    }

    fn clock_unselected(&mut self) {}
    fn clock(&mut self, input: CpuOutputPins) -> CpuInputPins {
        match input {
            CpuOutputPins::Read { addr } => match addr {
                0x0000..=0x3FFF => CpuInputPins {
                    data: self.bank_0()[addr as usize],
                    ..Default::default()
                },
                0x4000..=0x7FFF => CpuInputPins {
                    data: self.bank_1()[(addr - 0x4000) as usize],
                    ..Default::default()
                },
                0xA000..=0xBFFF => CpuInputPins {
                    data: if self.ram_enable {
                        self.ram[addr - 0xA000]
                    } else {
                        0
                    },
                    ..Default::default()
                },
                0x8000..=0x9FFF | 0xC000..=0xFFFF => panic!(),
            },
            CpuOutputPins::Write { addr, data } => {
                match addr {
                    0x0000..=0x1FFF => {
                        if data & 0x0F == 0xA {
                            self.ram_enable = true
                        } else {
                            self.ram_enable = false
                        }
                    }
                    0x2000..=0x3FFF => self.rom_bank_lower = data & 0x1F,
                    0x4000..=0x5FFF => self.rom_bank_upper = data & 0x03,
                    0x6000..=0x7FFF => self.mode_select = !(data == 0),
                    0xA000..=0xBFFF => {
                        if self.ram_enable {
                            self.ram[addr - 0xA000] = data
                        }
                    }
                    0x8000..=0x9FFF | 0xC000..=0xFFFF => panic!(),
                };
                Default::default()
            }
        }
    }
}

impl<R: ram::Ram> Mapper for Mbc1Generic<R> {}

mod ram {
    pub trait Ram: std::ops::IndexMut<u16, Output = u8> + Default {}

    #[derive(Default)]
    pub struct NullRam(u8);
    impl std::ops::Index<u16> for NullRam {
        type Output = u8;
        fn index(&self, _index: u16) -> &u8 {
            &0
        }
    }
    impl std::ops::IndexMut<u16> for NullRam {
        fn index_mut(&mut self, _index: u16) -> &mut u8 {
            &mut self.0
        }
    }

    impl Ram for NullRam {}

    pub struct BasicRam([u8; 0x2000]);
    impl Default for BasicRam {
        fn default() -> Self {
            BasicRam([0u8; 0x2000])
        }
    }
    impl std::ops::Index<u16> for BasicRam {
        type Output = u8;
        fn index(&self, index: u16) -> &u8 {
            &self.0[index as usize]
        }
    }
    impl std::ops::IndexMut<u16> for BasicRam {
        fn index_mut(&mut self, index: u16) -> &mut u8 {
            &mut self.0[index as usize]
        }
    }

    impl Ram for BasicRam {}
}
