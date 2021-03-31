//! A simple, but inaccurate implementation of the Gameboy monochrome PPU

use super::PpuOutputPins;

#[derive(Clone, Copy)]
pub struct PpuSimpleState {
    tile_data: [u8; 0x9800 - 0x8000],

    bg_map_1: [u8; 0x9C00 - 0x9800],
    bg_map_2: [u8; 0xA000 - 0x9C00],

    oam: [u8; 0xFEA0 - 0xFE00],

    lcdc: u8,
    stat: u8,
    scy: u8,
    scx: u8,
    ly: u8,
    lyc: u8,
    wy: u8,
    wx: u8,
    bgp: u8,
    obp0: u8,
    obp1: u8,
}

pub struct PpuSimple {
    pub state: PpuSimpleState,
    gen: std::pin::Pin<
        Box<
            dyn std::ops::Generator<
                (PpuSimpleState, super::PpuInputPins),
                Yield = (PpuSimpleState, super::PpuOutputPins),
                Return = !,
            >,
        >,
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
        };

        PpuSimple {
            state,
            gen: Box::pin(ppu_gen()),
        }
    }
}

fn ppu_gen() -> impl std::ops::Generator<
    (PpuSimpleState, super::PpuInputPins),
    Yield = (PpuSimpleState, super::PpuOutputPins),
    Return = !,
> {
    |t| {
        let (ppu, _) = t;
        loop {
            yield (ppu, Default::default());
        }
    }
}
