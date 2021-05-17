use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct LCDC: u8 {
        const LCD_ENABLE = 0x80;
        const WINDOW_TILEMAP_AREA = 0x40;
        const WINDOW_ENABLE = 0x20;
        const BG_TILE_DATA_AREA = 0x10;
        const BG_TILEMAP_AREA = 0x08;
        const OBJ_SIZE = 0x04;
        const OBJ_ENABLE = 0x02;
        const BG_ENABLE = 0x01;
        const BG_PRIORITY = 0x01;
    }
}

bitflags! {
    #[derive(Default)]
    pub struct STAT: u8 {
        const LYC_INTERRUPT_ENABLE = 0x40;
        const OAM_INTERRUPT_ENABLE = 0x20;
        const VBLANK_INTERRUPT_ENABLE = 0x10;
        const HBLANK_INTERRUPT_ENABLE = 0x08;
        const LYC_EQUALS_LY = 0x04;

        const MODE_0 = 0;
        const MODE_1 = 1;
        const MODE_2 = 2;
        const MODE_3 = 3;
    }
}

impl STAT {
    pub const MODE_BITMASK: STAT = STAT { bits: 0xFC };

    #[inline]
    pub fn set_mode(&mut self, mode: Self) {
        assert_matches!(
            mode,
            STAT::MODE_0 | STAT::MODE_1 | STAT::MODE_2 | STAT::MODE_3
        );
        *self &= Self::MODE_BITMASK;
        *self |= mode;
    }
}
