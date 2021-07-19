//! An implementation of the Gameboy monochrome PPU

use crate::cpu::{CpuInputPins, CpuOutputPins};

use super::{registers::*, PPU};
use std::{cell::RefCell, fmt::Debug, ops::GeneratorState, rc::Rc};

pub const FRAME_T_CYCLES: usize = 70224;

#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub pixels: [u32; 144 * 160],
    pub width: usize,
    pub height: usize,
}

#[derive(Clone)]
pub struct MonochromePpuState {
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

    frame: Rc<Frame>,
}

impl Debug for MonochromePpuState {
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

pub struct MonochromePpu {
    pub state: Rc<RefCell<MonochromePpuState>>,
    gen: std::pin::Pin<
        Box<
            dyn std::ops::Generator<
                Rc<RefCell<MonochromePpuState>>,
                Yield = Rc<RefCell<MonochromePpuState>>,
                Return = !,
            >,
        >,
    >,
}

impl MonochromePpu {
    pub fn new() -> Self {
        let state = MonochromePpuState {
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

            frame: Rc::new(Frame {
                pixels: [0; 144 * 160],
                width: 160,
                height: 144,
            }),
        };

        MonochromePpu {
            state: Rc::new(RefCell::new(state)),
            gen: Box::pin(ppu_gen()),
        }
    }
}

impl MonochromePpuState {
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

    /// Create an image displaying the entire current tile data, width, and height
    pub fn display_tile_data(&self) -> (Vec<u32>, usize, usize) {
        const TILE_COUNT: usize = (0x9800 - 0x8000) / 16;
        const ROW_LENGTH: usize = 16;
        const TILE_WIDTH: usize = 8;
        const IMAGE_WIDTH: usize = ROW_LENGTH * TILE_WIDTH;
        const ROWS: usize = TILE_COUNT / ROW_LENGTH;
        const IMAGE_HEIGHT: usize = ROWS * TILE_WIDTH;

        let mut image = vec![0; IMAGE_WIDTH * IMAGE_HEIGHT];
        let bgp = self.bgp;

        for row in 0..ROWS {
            let basey = TILE_WIDTH * row;
            for col in 0..ROW_LENGTH {
                let basex = TILE_WIDTH * col;

                let tile_id = row * ROW_LENGTH + col;
                for offy in 0..TILE_WIDTH {
                    let row_lo = self.tile_data[tile_id * 16 + 2 * offy + 0];
                    let row_hi = self.tile_data[tile_id * 16 + 2 * offy + 1];
                    for offx in 0..TILE_WIDTH {
                        let colorbit_lo = (row_lo << offx) >> 7;
                        let colorbit_hi = (row_hi << offx) >> 7;
                        let color_id = color::calculate_monochrome_color_id(
                            bgp,
                            (colorbit_hi << 1) | colorbit_lo,
                        );
                        let color = color::COLORS[color_id];

                        let imgx = basex + offx;
                        let imgy = basey + offy;
                        let offset = imgy * IMAGE_WIDTH + imgx;
                        image[offset] = color;
                    }
                }
            }
        }

        (image, IMAGE_WIDTH, IMAGE_HEIGHT)
    }
}

fn ppu_gen() -> impl std::ops::Generator<
    Rc<RefCell<MonochromePpuState>>,
    Yield = Rc<RefCell<MonochromePpuState>>,
    Return = !,
> {
    |mut ppu: Rc<RefCell<MonochromePpuState>>| loop {
        let mut frame = Frame {
            pixels: [0; 144 * 160],
            width: 160,
            height: 144,
        };

        // Drawing lines
        for line in 0..144 {
            ppu.borrow_mut().set_ly(line);

            let mut cycle = 0;
            // OAM Search (mode 2)
            ppu.borrow_mut().set_mode(2);
            for _ in 0..80 {
                cycle += 1;
                ppu = yield ppu;
            }

            // Drawing (mode 3)
            // TODO: this only draws the background for now
            ppu.borrow_mut().set_mode(3);
            let mut dot = 0;
            let mut screen_tile_x = 0;
            let mut x = ppu.borrow().scx;
            while dot < 160 {
                let (bg_fifo_lo, bg_fifo_hi) = {
                    let ppu = ppu.borrow();
                    let tilemap = if ppu.lcdc.contains(LCDC::BG_TILEMAP_AREA) {
                        ppu.bg_map_2
                    } else {
                        ppu.bg_map_1
                    };
                    let tile_data = ppu.tile_data;

                    let fetcher_x = ((ppu.scx / 8) + screen_tile_x) & 0x1F;
                    let fetcher_y = ppu.scy.wrapping_add(line) / 8;
                    let tile_idx = tilemap[fetcher_y as usize * 32 + fetcher_x as usize];

                    let tile_y = ppu.scy.wrapping_add(line) % 8;
                    if ppu.lcdc.contains(LCDC::BG_TILE_DATA_AREA) {
                        // $8000 method
                        let offset = tile_idx as usize * 16 + tile_y as usize * 2;
                        (tile_data[offset + 0], tile_data[offset + 1])
                    } else {
                        // $8800 method
                        let offset =
                            (0x1000 + (tile_idx as i8 as i16) * 16 + (tile_y as i16) * 2) as usize;
                        (tile_data[offset + 0], tile_data[offset + 1])
                    }
                };

                while x < 8 {
                    let bit = 7 - x;
                    x += 1;
                    let bg_color_hi = (bg_fifo_hi >> bit) & 1;
                    let bg_color_lo = (bg_fifo_lo >> bit) & 1;
                    let bg_color = (bg_color_hi << 1) | bg_color_lo;

                    let bg_color_rgb =
                        color::calculate_monochrome_color_id(ppu.borrow().bgp, bg_color);
                    frame.pixels[160 * line as usize + dot as usize] =
                        color::COLORS[bg_color_rgb as usize];
                    dot += 1;

                    cycle += 1;
                    ppu = yield ppu;
                }
                x = 0;
                screen_tile_x += 1;
            }

            // HBlank (mode 0)
            while cycle < 456 {
                cycle += 1;
                ppu = yield ppu;
            }
        }

        ppu.borrow_mut().frame = Rc::new(frame);

        // VBlank (mode 1)
        ppu.borrow_mut().set_mode(1);
        ppu.borrow_mut().vblank_irq = true;
        for line in 144..154 {
            ppu.borrow_mut().set_ly(line);
            for _ in 0usize..456 {
                ppu = yield ppu;
            }
        }
        ppu.borrow_mut().vblank_irq = false;
    }
}

impl PPU for MonochromePpu {
    type Frame = Frame;

    #[inline]
    fn perform_io(&mut self, input: CpuOutputPins) -> CpuInputPins {
        let mut state = self.state.borrow_mut();
        let data = match input {
            CpuOutputPins::Write { addr, data: v } => {
                match addr {
                    0x8000..=0x97FF => state.tile_data[addr as usize - 0x8000] = v,
                    0x9800..=0x9BFF => state.bg_map_1[addr as usize - 0x9800] = v,
                    0x9C00..=0x9FFF => state.bg_map_1[addr as usize - 0x9C00] = v,

                    0xFE00..=0xFE9F => state.oam[addr as usize - 0xFE00] = v,

                    0xFF40 => state.lcdc = LCDC::from_bits_truncate(v),
                    0xFF41 => {
                        state.stat = STAT::from_bits_truncate(v);
                        state.update_stat_interrupt();
                    }
                    0xFF42 => state.scy = v,
                    0xFF43 => state.scx = v,
                    0xFF44 => state.ly = v,
                    0xFF45 => state.lyc = v,
                    0xFF46 => (),
                    0xFF47 => state.bgp = v,
                    0xFF48 => state.obp0 = v,
                    0xFF49 => state.obp1 = v,
                    0xFF4A => state.wy = v,
                    0xFF4B => state.wx = v,
                    _ => panic!("{:?}", input),
                }
                0
            }
            CpuOutputPins::Read { addr } => match addr {
                0x8000..=0x97FF => state.tile_data[addr as usize - 0x8000],
                0x9800..=0x9BFF => state.bg_map_1[addr as usize - 0x9800],
                0x9C00..=0x9FFF => state.bg_map_1[addr as usize - 0x9C00],

                0xFE00..=0xFE9F => state.oam[addr as usize - 0xFE00],

                0xFF40 => state.lcdc.bits(),
                0xFF41 => state.stat.bits(),
                0xFF42 => state.scy,
                0xFF43 => state.scx,
                0xFF44 => state.ly,
                0xFF45 => state.lyc,
                0xFF46 => 0,
                0xFF47 => state.bgp,
                0xFF48 => state.obp0,
                0xFF49 => state.obp1,
                0xFF4A => state.wy,
                0xFF4B => state.wx,

                _ => panic!("{:?}", input),
            },
        };

        CpuInputPins {
            data,
            interrupt_40h: state.vblank_irq,
            interrupt_48h: state.stat_irq,
            ..Default::default()
        }
    }

    fn clock_t_state(&mut self) {
        // im not sure if theres a good way to borrow an object only for the duration of a generator run,
        // so instead i just clone the state in and out of the generator context. unfortunately this means
        // i have to use Rc<RefCell> to avoid doing huge copies hundreds of times a second
        self.state = match self.gen.as_mut().resume(self.state.clone()) {
            GeneratorState::Yielded(state) => state,
            GeneratorState::Complete(_) => unreachable!(),
        };
    }

    fn get_frame(&self) -> Frame {
        *self.state.borrow().frame
    }
}

pub mod color {
    pub const COLOR_BLACK: u32 = 0xFF000000;
    pub const COLOR_DARKGRAY: u32 = 0xFF777777;
    pub const COLOR_LIGHTGRAY: u32 = 0xFFAAAAAA;
    pub const COLOR_WHITE: u32 = 0xFFFFFFFF;

    pub const COLORS: [u32; 4] = [COLOR_WHITE, COLOR_LIGHTGRAY, COLOR_DARKGRAY, COLOR_BLACK];

    pub fn calculate_monochrome_color_id(palette: u8, pix: u8) -> usize {
        assert!(pix < 4);
        ((palette >> (pix * 2)) & 0x03) as usize
    }
}
