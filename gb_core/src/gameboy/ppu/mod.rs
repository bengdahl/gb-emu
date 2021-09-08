//! An implementation of the Gameboy monochrome PPU
pub mod color;
pub mod consts;
pub mod frame;
pub mod registers;

use frame::Frame;
use registers::{LCDC, STAT};
use std::{fmt::Debug, sync::Arc};

use crate::cpu::CpuOutputPins;

use super::Chip;

#[derive(Clone)]
pub struct Ppu {
    pub tile_data: [u8; 0x9800 - 0x8000],

    pub bg_map_1: [u8; 0x9C00 - 0x9800],
    pub bg_map_2: [u8; 0xA000 - 0x9C00],

    pub oam: [u8; 0xFEA0 - 0xFE00],

    pub lcdc: LCDC,
    pub stat: STAT,
    pub scy: u8,
    pub scx: u8,
    pub ly: u8,
    pub lyc: u8,
    pub wy: u8,
    pub wx: u8,
    pub bgp: u8,
    pub obp0: u8,
    pub obp1: u8,

    vblank_irq: bool,
    stat_irq: bool,

    frame: Arc<Frame>,
}

impl Debug for Ppu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MonochromePpuState")
            .field("LCDC", &self.lcdc)
            .field("STAT", &self.stat)
            .field("SCY", &self.scy)
            .field("SCX", &self.scx)
            .field("LY", &self.ly)
            .field("LYC", &self.lyc)
            .field("WY", &self.wy)
            .field("WX", &self.wx)
            .field("BGP", &self.bgp)
            .field("OBP0", &self.obp0)
            .field("OBP1", &self.obp1)
            .finish_non_exhaustive()
    }
}

impl Ppu {
    pub fn new() -> Self {
        Ppu {
            tile_data: [0u8; 0x9800 - 0x8000],

            bg_map_1: [0u8; 0x9C00 - 0x9800],
            bg_map_2: [0u8; 0xA000 - 0x9C00],

            oam: [0u8; 0xFEA0 - 0xFE00],

            lcdc: Default::default(),
            stat: Default::default(),
            scy: 0u8,
            scx: 0u8,
            ly: 0u8,
            lyc: 0u8,
            wy: 0u8,
            wx: 0u8,
            bgp: 0u8,
            obp0: 0u8,
            obp1: 0u8,

            vblank_irq: false,
            stat_irq: false,

            frame: Arc::new(Frame::new()),
        }
    }
}

impl Ppu {
    #[inline(always)]
    fn set_ly(&mut self, ly: u8) {
        debug_assert!(ly <= 153);
        self.ly = ly;
        self.stat.set(STAT::LYC_EQUALS_LY, self.ly == self.lyc);

        self.update_stat_interrupt();
    }

    #[inline(always)]
    fn set_mode(&mut self, mode: u8) {
        debug_assert!(mode <= 3);
        self.stat.set_mode(STAT::from_bits_truncate(mode));

        self.update_stat_interrupt();
    }

    #[inline(always)]
    fn update_stat_interrupt(&mut self) {
        let mode = self.stat & !STAT::MODE_BITMASK;

        let mode_int = match mode {
            STAT::MODE_0 if self.stat.contains(STAT::HBLANK_INTERRUPT_ENABLE) => true,
            STAT::MODE_1 if self.stat.contains(STAT::VBLANK_INTERRUPT_ENABLE) => true,
            STAT::MODE_2 if self.stat.contains(STAT::OAM_INTERRUPT_ENABLE) => true,
            _ => false,
        };

        let lyc_int = self
            .stat
            .contains(STAT::LYC_INTERRUPT_ENABLE | STAT::LYC_EQUALS_LY);

        self.stat_irq = mode_int | lyc_int;
    }

    /// Create an image displaying the entire current tile data, width, and height.
    ///
    /// The image is scaled a positive integer amount by `scale`, which defaults to 1.
    pub fn display_tile_data(&self, scale: impl Into<Option<usize>>) -> (Vec<u32>, usize, usize) {
        const TILE_COUNT: usize = (0x9800 - 0x8000) / 16;
        const ROW_LENGTH: usize = 16;
        const TILE_WIDTH: usize = 8;
        const IMAGE_WIDTH: usize = ROW_LENGTH * TILE_WIDTH;
        const ROWS: usize = TILE_COUNT / ROW_LENGTH;
        const IMAGE_HEIGHT: usize = ROWS * TILE_WIDTH;

        let scale = scale.into().unwrap_or(1);
        let mut image = vec![0; IMAGE_WIDTH * scale * IMAGE_HEIGHT * scale];
        let bgp = self.bgp;

        for row in 0..ROWS {
            let basey = TILE_WIDTH * row;
            for col in 0..ROW_LENGTH {
                let basex = TILE_WIDTH * col;

                let tile_id = row * ROW_LENGTH + col;
                for offy in 0..TILE_WIDTH {
                    let row_lo = self.tile_data[tile_id * 16 + 2 * offy + 0];
                    let row_hi = self.tile_data[tile_id * 16 + 2 * offy + 1];
                    for ypix in 0..scale {
                        for offx in 0..TILE_WIDTH {
                            let colorbit_lo = (row_lo << offx) >> 7;
                            let colorbit_hi = (row_hi << offx) >> 7;
                            let color_id = color::calculate_monochrome_color_id(
                                bgp,
                                (colorbit_hi << 1) | colorbit_lo,
                            );
                            let color = color::COLORS[color_id];

                            let imgy = (basey + offy) * scale + ypix;
                            for xpix in 0..scale {
                                let imgx = (basex + offx) * scale + xpix;
                                let offset = imgy * (IMAGE_WIDTH * scale) + imgx;
                                image[offset] = color;
                            }
                        }
                    }
                }
            }
        }

        (image, IMAGE_WIDTH * scale, IMAGE_HEIGHT * scale)
    }
}

impl Ppu {
    #[inline]
    pub fn perform_io(&mut self, input: CpuOutputPins, data: &mut u8, interrupt_request: &mut u8) {
        match input {
            CpuOutputPins::Write { addr, data: v } => match addr {
                0x8000..=0x97FF => self.tile_data[addr as usize - 0x8000] = v,
                0x9800..=0x9BFF => self.bg_map_1[addr as usize - 0x9800] = v,
                0x9C00..=0x9FFF => self.bg_map_2[addr as usize - 0x9C00] = v,

                0xFE00..=0xFE9F => self.oam[addr as usize - 0xFE00] = v,

                0xFF40 => self.lcdc = LCDC::from_bits_truncate(v),
                0xFF41 => {
                    self.stat = STAT::from_bits_truncate(v);
                    self.update_stat_interrupt();
                }
                0xFF42 => self.scy = v,
                0xFF43 => self.scx = v,
                0xFF44 => self.ly = v,
                0xFF45 => self.lyc = v,
                0xFF46 => (),
                0xFF47 => self.bgp = v,
                0xFF48 => self.obp0 = v,
                0xFF49 => self.obp1 = v,
                0xFF4A => self.wy = v,
                0xFF4B => self.wx = v,
                _ => (),
            },
            CpuOutputPins::Read { addr } => match addr {
                0x8000..=0x97FF => *data = self.tile_data[addr as usize - 0x8000],
                0x9800..=0x9BFF => *data = self.bg_map_1[addr as usize - 0x9800],
                0x9C00..=0x9FFF => *data = self.bg_map_2[addr as usize - 0x9C00],

                0xFE00..=0xFE9F => *data = self.oam[addr as usize - 0xFE00],

                0xFF40 => *data = self.lcdc.bits(),
                0xFF41 => *data = self.stat.bits(),
                0xFF42 => *data = self.scy,
                0xFF43 => *data = self.scx,
                0xFF44 => *data = self.ly,
                0xFF45 => *data = self.lyc,
                0xFF46 => *data = 0,
                0xFF47 => *data = self.bgp,
                0xFF48 => *data = self.obp0,
                0xFF49 => *data = self.obp1,
                0xFF4A => *data = self.wy,
                0xFF4B => *data = self.wx,

                _ => (),
            },
        };

        let mut irq = *interrupt_request;
        if self.vblank_irq {
            irq |= 1 << 0;
        } else {
            irq &= !(1 << 0);
        }

        if self.stat_irq {
            irq |= 1 << 1;
        } else {
            irq &= !(1 << 1);
        }

        *interrupt_request = irq;
    }

    pub fn clock_t_state(&mut self) {
        todo!()
    }

    pub fn get_frame(&self) -> Frame {
        *self.frame
    }
}

impl Chip for Ppu {
    fn clock(&mut self, input: CpuOutputPins, data: &mut u8, interrupt_request: &mut u8) {
        self.perform_io(input, data, interrupt_request);
        for _ in 0..4 {
            self.clock_t_state();
        }
    }
}
