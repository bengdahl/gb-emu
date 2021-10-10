use super::super::registers::{OamEntry, OamEntryFlags, LCDC};

use super::PpuState;

pub struct BgPixelFifo {
    pixels: ShiftRegister<Pixel, 16>,
    tile_map_offset: TileCounter,
    state: FifoState,
}

impl BgPixelFifo {
    pub fn new() -> Self {
        Self {
            pixels: ShiftRegister::new(),
            tile_map_offset: TileCounter::Bg { x_counter: 0 },
            state: FifoState::FetchTile,
        }
    }

    pub fn set_tile_map_offset(&mut self, tile_map_offset: TileCounter) {
        self.tile_map_offset = tile_map_offset;
    }

    pub fn reset_fetcher(&mut self) {
        self.state = FifoState::FetchTile;
    }

    pub fn clear(&mut self) {
        self.reset_fetcher();
        self.pixels.clear();
    }

    /// Each FIFO cycle takes 2 PPU cycles
    pub fn clock(&mut self, state: &PpuState) {
        match self.state {
            FifoState::FetchTile => {
                self.state = FifoState::FetchTileDataLow {
                    tile_data_index: {
                        let tile_no = self.tile_map_offset.get_tile_number(state);
                        let tile_addr = state.bg_tile_data_address(tile_no);
                        let tile_line_offset = match self.tile_map_offset {
                            TileCounter::Bg { .. } => 2 * ((state.ly + state.scy) % 8) as usize,
                            TileCounter::Window { window_line, .. } => {
                                2 * (window_line % 8) as usize
                            }
                        };
                        tile_addr + tile_line_offset
                    },
                }
            }

            FifoState::FetchTileDataLow { tile_data_index } => {
                self.state = FifoState::FetchTileDataHigh {
                    tile_data_index,
                    tile_data_low: state.tile_data[tile_data_index],
                }
            }

            FifoState::FetchTileDataHigh {
                tile_data_index,
                tile_data_low,
            } => {
                self.state = FifoState::ReadyToPush {
                    tile_data_low,
                    tile_data_high: state.tile_data[tile_data_index + 1],
                }
            }

            FifoState::ReadyToPush {
                tile_data_low,
                tile_data_high,
            } => {
                if self.pixels.len() <= 8 {
                    for bit in (0..8).rev() {
                        let pix_low = (tile_data_low >> bit) & 1;
                        let pix_high = (tile_data_high >> bit) & 1;
                        self.pixels
                            .push(Pixel {
                                color: (pix_high << 1) | pix_low,
                                ..Default::default()
                            })
                            .unwrap();
                    }
                    self.tile_map_offset.increment();
                    self.state = FifoState::FetchTile;
                }
            }
        }
    }

    pub fn pop_pixel(&mut self) -> Option<Pixel> {
        if self.pixels.len() > 8 {
            self.pixels.pop()
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileCounter {
    Bg { x_counter: u16 },
    Window { x_counter: u16, window_line: u16 },
}

impl TileCounter {
    fn get_tile_number(&self, state: &PpuState) -> u8 {
        match self {
            &TileCounter::Bg { x_counter } => state.get_bg_tile_number(
                (state.ly.wrapping_add(state.scy) as u16 / 8 * 32
                    + ((state.scx as u16 / 8 + x_counter) & 0x1F))
                    & 0x3FF,
            ),
            &TileCounter::Window {
                x_counter,
                window_line,
            } => state.get_window_tile_number(window_line / 8 * 32 + x_counter),
        }
    }

    fn increment(&mut self) {
        match self {
            TileCounter::Bg { ref mut x_counter } => {
                *x_counter += 1;
            }
            TileCounter::Window {
                ref mut x_counter, ..
            } => {
                *x_counter += 1;
            }
        }
    }
}

pub struct SpritePixelFifo {
    pixels: ShiftRegister<Pixel, 8>,
    sprite: Option<super::OamEntry>,
    state: FifoState,
}

impl SpritePixelFifo {
    pub fn new() -> Self {
        SpritePixelFifo {
            pixels: ShiftRegister::new(),
            sprite: None,
            state: FifoState::FetchTile,
        }
    }

    pub fn load_sprite(&mut self, sprite: OamEntry) {
        self.sprite = Some(sprite);
    }

    pub fn clock(&mut self, state: &mut PpuState) {
        match self.state {
            FifoState::FetchTile => match self.sprite {
                None => (),
                Some(sprite) => {
                    self.state = FifoState::FetchTileDataLow {
                        tile_data_index: {
                            let sprite_line = state.ly - self.sprite.unwrap().ypos + 16;
                            if sprite.flags.contains(OamEntryFlags::Y_FLIP) {
                                if state.lcdc.contains(LCDC::OBJ_SIZE) {
                                    // For y-flipped 8x16 sprites, we want to draw the second tile's
                                    // data first
                                    if sprite_line < 8 {
                                        state.sprite_tile_data_address(sprite.tile + 1)
                                            + 2 * (7 - sprite_line) as usize
                                    } else {
                                        state.sprite_tile_data_address(sprite.tile)
                                            + 2 * (7 - sprite_line + 8) as usize
                                    }
                                } else {
                                    state.sprite_tile_data_address(sprite.tile)
                                        + 2 * (7 - sprite_line) as usize
                                }
                            } else {
                                // For non-y-flipped sprites, the line offset will naturally roll
                                // over into the next tile.
                                state.sprite_tile_data_address(sprite.tile)
                                    + 2 * sprite_line as usize
                            }
                        },
                    }
                }
            },

            FifoState::FetchTileDataLow { tile_data_index } => {
                self.state = FifoState::FetchTileDataHigh {
                    tile_data_index,
                    tile_data_low: state.tile_data[tile_data_index],
                };
            }

            FifoState::FetchTileDataHigh {
                tile_data_index,
                tile_data_low,
            } => {
                self.state = FifoState::ReadyToPush {
                    tile_data_low,
                    tile_data_high: state.tile_data[tile_data_index + 1],
                };
            }

            FifoState::ReadyToPush {
                tile_data_low,
                tile_data_high,
            } => {
                for i in 0..8 {
                    let (pix_low, pix_high) =
                        if self.sprite.unwrap().flags.contains(OamEntryFlags::X_FLIP) {
                            let pix_low = (tile_data_low >> i) & 1;
                            let pix_high = (tile_data_high >> i) & 1;
                            (pix_low, pix_high)
                        } else {
                            let pix_low = (tile_data_low >> (7 - i)) & 1;
                            let pix_high = (tile_data_high >> (7 - i)) & 1;
                            (pix_low, pix_high)
                        };
                    let prepared_pixel = Pixel {
                        color: (pix_high << 1) | pix_low,
                        palette: if self
                            .sprite
                            .unwrap()
                            .flags
                            .contains(super::OamEntryFlags::PALETTE_OBP1)
                        {
                            1
                        } else {
                            0
                        },
                        bg_priority: self
                            .sprite
                            .unwrap()
                            .flags
                            .contains(super::OamEntryFlags::BG_PRIORITY),
                        sprite_priority: false,
                    };

                    // Avoid drawing on top of already visible sprite pixels
                    if let Some(pix) = self.pixels.get_mut(i) {
                        if pix.color != 0b00 {
                            continue;
                        } else {
                            // If the pixel is transparent, we can still overwrite it
                            *pix = prepared_pixel;
                        }
                    } else {
                        self.pixels.push(prepared_pixel).unwrap();
                    }
                }

                self.state = FifoState::FetchTile;
                self.sprite = None;
            }
        }
    }

    pub fn pop_pixel(&mut self) -> Pixel {
        // Output a transparent pixel if the fifo is empty
        self.pixels.pop().unwrap_or(Pixel {
            color: 0,
            ..Default::default()
        })
    }
}

enum FifoState {
    FetchTile,
    FetchTileDataLow {
        tile_data_index: usize,
    },
    FetchTileDataHigh {
        tile_data_index: usize,
        tile_data_low: u8,
    },
    ReadyToPush {
        tile_data_low: u8,
        tile_data_high: u8,
    },
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Pixel {
    /// Pixel color (palette index)
    pub color: u8,
    /// Palette (0-1 on DMG, 0-7 on CGB), only applies to sprites on DMG
    pub palette: u8,
    /// Sprite priority (only relevant on CGB)
    pub sprite_priority: bool,
    /// BG Priority (flag bit 7 of sprites)
    pub bg_priority: bool,
}

struct ShiftRegister<T: Default + Clone + Copy, const N: usize> {
    data: [T; N],
    /// Index of the front of the queue
    i: usize,
    /// Number of elements in the queue
    len: usize,
}

impl<T: Default + Clone + Copy, const N: usize> ShiftRegister<T, N> {
    fn new() -> Self {
        ShiftRegister {
            data: [Default::default(); N],
            i: 0,
            len: 0,
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn is_full(&self) -> bool {
        self.len == N
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn push(&mut self, v: T) -> Result<(), T> {
        if self.is_full() {
            return Err(v);
        }

        let index = (self.i + self.len) % N;
        self.data[index] = v;
        self.len += 1;
        Ok(())
    }

    fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let r = self.data[self.i].clone();
        self.i = (self.i + 1) % N;
        self.len -= 1;
        Some(r)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len {
            return None;
        }
        Some(&mut self.data[(self.i + index) % N])
    }

    fn clear(&mut self) {
        self.len = 0;
    }
}
