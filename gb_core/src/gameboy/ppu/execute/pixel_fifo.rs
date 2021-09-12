use super::PpuState;

pub struct PixelFifo {
    pixels: ShiftRegister<Pixel, 16>,
    tile_map_offset: TileMapOffset,
    state: FifoState,
}

impl PixelFifo {
    pub fn new() -> Self {
        Self {
            pixels: ShiftRegister::new(),
            tile_map_offset: TileMapOffset::Bg(0),
            state: FifoState::FetchTile,
        }
    }

    pub fn set_tile_map_offset(&mut self, tile_map_offset: TileMapOffset) {
        self.tile_map_offset = tile_map_offset;
    }

    pub fn clear(&mut self) {
        self.pixels.clear();
        self.state = FifoState::FetchTile;
    }

    /// Each FIFO cycle takes 2 PPU cycles
    pub fn clock_bg(&mut self, state: &PpuState) {
        match self.state {
            FifoState::FetchTile => {
                self.state = FifoState::FetchTileDataLow {
                    tile_no: match self.tile_map_offset {
                        TileMapOffset::Bg(off) => state.get_bg_tile_number(off),
                        TileMapOffset::Window(off, _) => state.get_window_tile_number(off),
                    },
                }
            }

            FifoState::FetchTileDataLow { tile_no } => {
                self.state = FifoState::FetchTileDataHigh {
                    tile_no,
                    tile_data_low: {
                        let tile_addr = state.tile_data_address(tile_no);
                        let tile_data_offset = match self.tile_map_offset {
                            TileMapOffset::Bg(_) => 2 * ((state.ly + state.scy) % 8) as usize,
                            TileMapOffset::Window(_, window_line) => 2 * (window_line % 8) as usize,
                        };
                        state.tile_data[tile_addr + tile_data_offset]
                    },
                }
            }

            FifoState::FetchTileDataHigh {
                tile_no,
                tile_data_low,
            } => {
                self.state = FifoState::ReadyToPush {
                    tile_data_low,
                    tile_data_high: {
                        let tile_addr = state.tile_data_address(tile_no);
                        let tile_data_offset = match self.tile_map_offset {
                            TileMapOffset::Bg(_) => 2 * ((state.ly + state.scy) % 8) as usize,
                            TileMapOffset::Window(_, window_line) => 2 * (window_line % 8) as usize,
                        };
                        state.tile_data[tile_addr + tile_data_offset + 1]
                    },
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
                        let _ = self.pixels.push(Pixel {
                            color: (pix_high << 1) | pix_low,
                            ..Default::default()
                        });
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
pub enum TileMapOffset {
    Bg(u16),
    Window(u16, u8),
}

impl TileMapOffset {
    fn increment(&mut self) {
        match self {
            TileMapOffset::Bg(ref mut off) => {
                *off += 1;
            }
            TileMapOffset::Window(ref mut off, _) => {
                *off += 1;
            }
        }
    }
}

enum FifoState {
    FetchTile,
    FetchTileDataLow {
        tile_no: u8,
    },
    FetchTileDataHigh {
        tile_no: u8,
        tile_data_low: u8,
    },
    ReadyToPush {
        tile_data_low: u8,
        tile_data_high: u8,
    },
}

#[derive(Default, Clone, Copy)]
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

    fn clear(&mut self) {
        self.len = 0;
    }
}
