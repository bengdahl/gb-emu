//! An implementation of the Gameboy monochrome PPU

#[derive(Clone)]
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

    vblank_irq: bool,
    stat_irq: bool,

    frame: super::Frame,
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

            frame: super::Frame {
                pixels: vec![0; 144 * 160],
                width: 160,
                height: 144,
            },
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
        // Drawing lines
        for line in 0..144 {
            ppu.set_ly(line);

            let mut dot = 0;
            // OAM Search (mode 2)
            ppu.set_mode(2);
            for _ in 0..80 {
                dot += 1;
                ppu = yield ppu;
            }

            // Drawing (mode 3)
            ppu.set_mode(3);
            // TODO: this loop has variable duration
            for _ in 0..168 {
                dot += 1;
                ppu = yield ppu;
            }

            // HBlank (mode 0)
            while dot < 456 {
                dot += 1;
                ppu = yield ppu;
            }
        }

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
