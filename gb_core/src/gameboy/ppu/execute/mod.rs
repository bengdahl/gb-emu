mod pixel_fifo;

use crate::{cpu::CpuOutputPins, gameboy::ppu::color};

use self::pixel_fifo::Pixel;

use super::{
    frame::Frame,
    registers::{OamEntry, OamEntryFlags, LCDC, STAT},
};
use std::{ops::Generator, pin::Pin};

pub struct PpuState {
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

    pub frame: Box<Frame>,
    // Double-buffer the frames to prevent tearing
    back_frame: Box<Frame>,
}

impl std::fmt::Debug for PpuState {
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

impl PpuState {
    pub fn new() -> Self {
        PpuState {
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

            frame: Box::new(Frame::new()),
            back_frame: Box::new(Frame::new()),
        }
    }

    /// Returns the nth OAM entry
    ///
    /// # Panics
    /// Panics if `index` > 40
    pub fn oam(&self, index: usize) -> OamEntry {
        assert!(index <= 40);
        OamEntry {
            ypos: self.oam[index * 4 + 0],
            xpos: self.oam[index * 4 + 1],
            tile: self.oam[index * 4 + 2],
            flags: OamEntryFlags::from_bits_truncate(self.oam[index * 4 + 3]),
        }
    }

    fn sprite_height(&self) -> u8 {
        if self.lcdc.contains(LCDC::OBJ_SIZE) {
            16
        } else {
            8
        }
    }

    /// Return the BG tile data at the given offset, taking into account the addressing mode
    ///
    /// # Panics
    /// Panics if `offset` >= 0x400
    fn get_bg_tile_number(&self, offset: u16) -> u8 {
        if self.lcdc.contains(LCDC::BG_TILEMAP_AREA) {
            self.bg_map_2[offset as usize]
        } else {
            self.bg_map_1[offset as usize]
        }
    }

    /// Return the window tile data at the given offset, taking into account the addressing mode
    ///
    /// # Panics
    /// Panics if `offset` >= 0x400
    fn get_window_tile_number(&self, offset: u16) -> u8 {
        if self.lcdc.contains(LCDC::WINDOW_TILEMAP_AREA) {
            self.bg_map_2[offset as usize]
        } else {
            self.bg_map_1[offset as usize]
        }
    }

    /// Return the index of the first byte of the tile data for tile `n`
    fn tile_data_address(&self, tile_no: u8) -> usize {
        if self.lcdc.contains(LCDC::BG_TILE_DATA_AREA) {
            tile_no as usize * 16
        } else {
            0x1000 + (tile_no as i8 as i16 * 16) as usize
        }
    }

    fn put_pixel(&mut self, bg_pix: Pixel, x: usize, y: usize) {
        assert!(x < 160);
        assert!(y < 144);
        let color_id = color::calculate_monochrome_color_id(self.bgp, bg_pix.color);
        self.back_frame[(x, y)] = color::COLORS[color_id];
    }

    fn swap_frames(&mut self) {
        std::mem::swap(&mut self.back_frame, &mut self.frame);
    }
}

impl PpuState {
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
    pub fn update_stat_interrupt(&mut self) {
        let mode = self.stat.mode();

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
}

pub type PpuGenerator =
    Pin<Box<dyn Generator<Box<PpuState>, Yield = Box<PpuState>, Return = !> + Send + Sync>>;

pub fn gen() -> PpuGenerator {
    Box::pin(|mut state: Box<PpuState>| {
        macro_rules! ppu_yield {
            () => {
                state = yield state
            };
        }

        loop {
            let mut bg_fifo = pixel_fifo::PixelFifo::new();
            // The window is rendered if ly==wy at any point during the frame
            let mut wy_passed = false;
            // Number of completed scanlines containing any window pixels
            let mut window_lines = 0;
            for scanline in 0..144 {
                state.set_ly(scanline);
                if state.ly == state.wy {
                    wy_passed = true;
                }

                // OAM Search
                state.set_mode(2);
                let mut sprite_buffer = [OamEntry::default(); 10];
                let mut sprite_buffer_len = 0;
                for entry in 0..40 {
                    if sprite_buffer_len < 10 {
                        let entry = state.oam(entry);
                        if entry.xpos > 0
                            && scanline + 16 >= entry.ypos
                            && scanline + 16 < entry.ypos + state.sprite_height()
                        {
                            sprite_buffer[sprite_buffer_len] = entry;
                            sprite_buffer_len += 1;
                        }
                    }
                    ppu_yield!();
                    ppu_yield!();
                }

                // Drawing
                state.set_mode(3);
                // 80 cycles have passed already
                let mut cycles = 80;
                bg_fifo.set_tile_map_offset(pixel_fifo::TileMapOffset::Bg(
                    state.ly.wrapping_add(state.scy) as u16 / 8 * 32 + state.scx as u16 / 8,
                ));
                // Discard the first SCX % 8 pixels
                let mut x = -(state.scx as isize % 8);
                let mut inside_window = false;
                while x < 160 {
                    if cycles % 2 == 0 {
                        bg_fifo.clock_bg(&mut state);
                    }
                    if let Some(pixel) = bg_fifo.pop_pixel() {
                        if x >= 0 {
                            state.put_pixel(pixel, x as usize, scanline as usize);
                        }
                        // Check if we're about to enter the window
                        if state.lcdc.contains(LCDC::WINDOW_ENABLE)
                            && wy_passed
                            && x >= state.wx as isize - 7
                            && !inside_window
                        {
                            bg_fifo.clear();
                            bg_fifo.set_tile_map_offset(pixel_fifo::TileMapOffset::Window(
                                window_lines / 8 * 32,
                                window_lines as u8,
                            ));
                            inside_window = true;
                        }
                        x += 1;
                    }
                    ppu_yield!();
                    cycles += 1;
                }
                if wy_passed {
                    window_lines += 1;
                }

                // HBlank
                state.set_mode(0);
                bg_fifo.clear();
                while cycles < 456 {
                    ppu_yield!();
                    cycles += 1;
                }
            }

            // VBlank
            state.set_mode(1);
            state.swap_frames();
            state.vblank_irq = true;
            for scanline in 144..154 {
                state.set_ly(scanline);
                for _dot in 0..456 {
                    ppu_yield!()
                }
            }
            state.vblank_irq = false;
        }
    })
}
