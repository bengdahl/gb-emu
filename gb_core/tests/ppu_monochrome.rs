use gb_core::gameboy::ppu::{monochrome, registers::*, PPU};

fn set_tile_singlecolor(ppu: &mut monochrome::MonochromePpu, tile_idx: usize, color: u8) {
    assert!(color <= 3);

    let offset = tile_idx * 16;
    let color_high = color >> 1;
    let color_low = color & 1;
    for i in 0..16 {
        if i % 2 == 0 {
            ppu.tile_data[offset + i] = 0xff * color_low;
        } else {
            ppu.tile_data[offset + i] = 0xff * color_high;
        }
    }
}

fn advance_frame(ppu: &mut monochrome::MonochromePpu) {
    for _ in 0..monochrome::FRAME_T_CYCLES {
        ppu.clock_t_state();
    }
}

#[test]
fn ppu_singlecolor() {
    let mut ppu = monochrome::MonochromePpu::new();

    ppu.bg_map_1.fill(0);
    ppu.lcdc = LCDC::LCD_ENABLE | LCDC::BG_ENABLE | LCDC::BG_TILE_DATA_AREA;
    ppu.bgp = 0b11100100;

    for color in [0b00, 0b01, 0b10, 0b11] {
        println!("color: {:b}", color);
        set_tile_singlecolor(&mut ppu, 0, color);
        advance_frame(&mut ppu);
        let frame = ppu.get_frame();
        frame.pixels.iter().for_each(|&pix| {
            assert_eq!(
                pix,
                monochrome::color::COLORS
                    [monochrome::color::calculate_monochrome_color_id(ppu.bgp, color) as usize]
            )
        });
    }
}

#[test]
fn ppu_bgp() {
    let mut ppu = monochrome::MonochromePpu::new();

    ppu.lcdc = LCDC::LCD_ENABLE | LCDC::BG_ENABLE | LCDC::BG_TILE_DATA_AREA;
    set_tile_singlecolor(&mut ppu, 0, 0b00);
    set_tile_singlecolor(&mut ppu, 1, 0b01);
    set_tile_singlecolor(&mut ppu, 2, 0b10);
    set_tile_singlecolor(&mut ppu, 3, 0b11);
    for i in 0..0x400 {
        ppu.bg_map_1[i] = (i % 4) as u8;
    }

    for bgp in 0..=0xFF {
        ppu.bgp = bgp;
        advance_frame(&mut ppu);

        let frame = ppu.get_frame();

        for color in 0..=3 {
            let i = color as usize * 8; // 8 pixel wide tiles
            assert_eq!(
                frame.pixels[i],
                monochrome::color::COLORS
                    [monochrome::color::calculate_monochrome_color_id(bgp, color)]
            );
        }
    }
}

#[test]
fn calculate_monochrome_color() {
    let tests = vec![
        (
            0b11100100,
            vec![(0b00, 0b00), (0b01, 0b01), (0b10, 0b10), (0b11, 0b11)],
        ),
        (
            0b00011011,
            vec![(0b00, 0b11), (0b01, 0b10), (0b10, 0b01), (0b11, 0b00)],
        ),
    ];

    for (bgp, cases) in tests {
        for (color, expected) in cases {
            let actual = monochrome::color::calculate_monochrome_color_id(bgp, color);
            assert_eq!(
                expected, actual,
                "bgp: {:b}, color: {:b}, expected: {:X}, actual: {:X}",
                bgp, color, expected, actual
            );
        }
    }
}
