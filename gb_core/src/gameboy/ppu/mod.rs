//! An implementation of the Gameboy monochrome PPU
pub mod color;
pub mod consts;
mod execute;
pub mod frame;
pub mod registers;

use frame::Frame;
use std::ops::{Deref, DerefMut, GeneratorState};

use crate::cpu::CpuOutputPins;

use self::execute::PpuState;

use super::Chip;

pub struct Ppu {
    state: Option<Box<PpuState>>,
    gen: execute::PpuGenerator,
}

impl Ppu {
    pub fn new() -> Self {
        Ppu {
            state: Some(Box::new(PpuState::new())),
            gen: execute::gen(),
        }
    }
}

impl Deref for Ppu {
    type Target = PpuState;

    fn deref(&self) -> &Self::Target {
        self.state.as_ref().unwrap()
    }
}

impl DerefMut for Ppu {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.state.as_mut().unwrap()
    }
}

impl Ppu {
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
    pub fn clock_t_state(&mut self) {
        match self.gen.as_mut().resume(self.state.take().unwrap()) {
            GeneratorState::Yielded(state) => self.state = Some(state),
            GeneratorState::Complete(_) => unreachable!(),
        }
    }

    pub fn get_frame(&self) -> Box<Frame> {
        self.frame.clone()
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
