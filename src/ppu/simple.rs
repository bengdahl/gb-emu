//! An implementation of the Gameboy monochrome PPU

use super::{Frame, PpuInputPins, PpuOutputPins};
use std::{ops::GeneratorState, rc::Rc};

#[derive(Clone)]
pub struct PpuSimpleState {
    pub tile_data: [u8; 0x9800 - 0x8000],

    pub bg_map_1: [u8; 0x9C00 - 0x9800],
    pub bg_map_2: [u8; 0xA000 - 0x9C00],

    pub oam: [u8; 0xFEA0 - 0xFE00],

    pub lcdc: u8,
    pub stat: u8,
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

    frame: Rc<super::Frame>,
}

pub struct PpuSimple {
    pub state: PpuSimpleState,
    gen: std::pin::Pin<
        Box<dyn std::ops::Generator<PpuSimpleState, Yield = PpuSimpleState, Return = !>>,
    >,
}

impl PpuSimple {
    pub fn new() -> Self {
        let state = PpuSimpleState {
            tile_data: [0u8; 0x9800 - 0x8000],

            bg_map_1: [0u8; 0x9C00 - 0x9800],
            bg_map_2: [0u8; 0xA000 - 0x9C00],

            oam: [0u8; 0xFEA0 - 0xFE00],

            lcdc: 0u8,
            stat: 0u8,
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

            frame: Rc::new(super::Frame {
                pixels: vec![0; 144 * 160],
                width: 160,
                height: 144,
            }),
        };

        PpuSimple {
            state,
            gen: Box::pin(ppu_gen()),
        }
    }
}

impl PpuSimpleState {
    fn set_ly(&mut self, ly: u8) {
        assert!(ly <= 153);
        self.ly = ly;
        if self.ly == self.lyc {
            self.stat |= 0x04;
        } else {
            self.stat &= !0x04;
        }

        self.update_stat_interrupt();
    }

    fn set_mode(&mut self, mode: u8) {
        assert!(mode <= 3);
        self.stat &= 0xFC;
        self.stat |= mode;

        self.update_stat_interrupt();
    }

    fn update_stat_interrupt(&mut self) {
        let mode = self.stat & 0x03;

        let mode_int = match mode {
            0 if self.stat & 0x08 != 0 => true,
            1 if self.stat & 0x10 != 0 => true,
            2 if self.stat & 0x20 != 0 => true,
            _ => false,
        };

        let lyc_int = self.stat & 0x04 != 0 && self.stat & 0x40 != 0;

        self.stat_irq = mode_int | lyc_int;
    }
}

fn ppu_gen() -> impl std::ops::Generator<PpuSimpleState, Yield = PpuSimpleState, Return = !> {
    |mut ppu: PpuSimpleState| loop {
        let mut frame = Frame {
            pixels: vec![0; 144 * 160],
            width: 160,
            height: 144,
        };

        // Drawing lines
        for line in 0..144 {
            ppu.set_ly(line);

            let mut cycle = 0;
            // OAM Search (mode 2)
            ppu.set_mode(2);
            for _ in 0..80 {
                cycle += 1;
                ppu = yield ppu;
            }

            // Drawing (mode 3)
            // TODO: this only draws the background for now
            ppu.set_mode(3);
            let mut dot = 0;
            let mut screen_tile_x = 0;
            let mut x = ppu.scx;
            while dot < 160 {
                let tilemap = if ppu.lcdc & 0x08 != 0 {
                    &ppu.bg_map_2
                } else {
                    &ppu.bg_map_1
                };

                let fetcher_x = ((ppu.scx / 8) + screen_tile_x) & 0x1F;
                let fetcher_y = ((ppu.scy + line) & 0xFF) / 8;
                let tile_idx = tilemap[(fetcher_y * 32 + fetcher_x) as usize];

                let tile_y = (ppu.scy + line) % 8;

                let (bg_fifo_lo, bg_fifo_hi) = if ppu.lcdc & 0x10 != 0 {
                    // $8000 method
                    let offset = (tile_idx * 16 + tile_y * 2) as usize;
                    (ppu.tile_data[offset + 0], ppu.tile_data[offset + 1])
                } else {
                    // $8800 method
                    let offset =
                        (0x1000 + (tile_idx as i8 as i16) * 16 + (tile_y as i16) * 2) as usize;
                    (ppu.tile_data[offset + 0], ppu.tile_data[offset + 1])
                };

                while x < 8 {
                    let bit = 7 - x;
                    x += 1;
                    let bg_color_hi = (bg_fifo_hi >> bit) & 1;
                    let bg_color_lo = (bg_fifo_lo >> bit) & 1;
                    let bg_color = (bg_color_hi << 1) | bg_color_lo;

                    let bg_color_rgb = calculate_monochrome_color(ppu.bgp, bg_color);
                    frame.pixels[(160 * line + dot) as usize] = bg_color_rgb;
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

        ppu.frame = Rc::new(frame);

        // VBlank (mode 1)
        ppu.set_mode(1);
        ppu.vblank_irq = true;
        for line in 144..154 {
            ppu.set_ly(line);
            for _dot in 0..456 {
                ppu = yield ppu;
            }
        }
        ppu.vblank_irq = false;
    }
}

impl super::PPU for PpuSimple {
    fn clock(&mut self, input: PpuInputPins) -> PpuOutputPins {
        let data = if !input.is_read {
            let v = input.data;
            match input.addr {
                0x8000..=0x97FF => self.state.tile_data[input.addr as usize - 0x8000] = v,
                0x9800..=0x9BFF => self.state.bg_map_1[input.addr as usize - 0x9800] = v,
                0x9C00..=0x9FFF => self.state.bg_map_1[input.addr as usize - 0x9C00] = v,

                0xFE00..=0xFE9F => self.state.oam[input.addr as usize - 0xFE00] = v,

                0xFF40 => self.state.lcdc = v,
                0xFF41 => {
                    self.state.stat = (v & 0xFC) | 0x80;
                    self.state.update_stat_interrupt();
                }
                0xFF42 => self.state.scy = v,
                0xFF43 => self.state.scx = v,
                0xFF44 => self.state.ly = v,
                0xFF45 => self.state.lyc = v,
                // 0xFF46 => DMA,
                0xFF47 => self.state.bgp = v,
                0xFF48 => self.state.obp0 = v,
                0xFF49 => self.state.obp1 = v,
                0xFF4A => self.state.wy = v,
                0xFF4B => self.state.wx = v,
                _ => panic!(),
            }
            0
        } else {
            match input.addr {
                0x8000..=0x97FF => self.state.tile_data[input.addr as usize - 0x8000],
                0x9800..=0x9BFF => self.state.bg_map_1[input.addr as usize - 0x9800],
                0x9C00..=0x9FFF => self.state.bg_map_1[input.addr as usize - 0x9C00],

                0xFE00..=0xFE9F => self.state.oam[input.addr as usize - 0xFE00],

                0xFF40 => self.state.lcdc,
                0xFF41 => self.state.stat | 0x80,
                0xFF42 => self.state.scy,
                0xFF43 => self.state.scx,
                0xFF44 => self.state.ly,
                0xFF45 => self.state.lyc,
                // 0xFF46 => DMA,
                0xFF47 => self.state.bgp,
                0xFF48 => self.state.obp0,
                0xFF49 => self.state.obp1,
                0xFF4A => self.state.wy,
                0xFF4B => self.state.wx,

                _ => panic!(),
            }
        };

        self.state = match self.gen.as_mut().resume(self.state.clone()) {
            GeneratorState::Yielded(state) => state,
            GeneratorState::Complete(_) => unreachable!(),
        };

        PpuOutputPins {
            data,
            vblank_interrupt: self.state.vblank_irq,
            stat_interrupt: self.state.stat_irq,
        }
    }

    fn get_frame(&self) -> &Frame {
        &self.state.frame
    }
}

fn calculate_monochrome_color(palette: u8, pix: u8) -> u32 {
    assert!(pix < 4);
    let color = (palette >> (pix * 2)) & 0x03;
    match color {
        0 => 0x00FFFFFF,
        1 => 0x00AAAAAA,
        2 => 0x00777777,
        3 => 0x00000000,
        _ => unreachable!(),
    }
}
