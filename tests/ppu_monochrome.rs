use gb_core::gameboy::ppu::{self, registers::*, PPU};

fn set_tile_singlecolor(ppu: &mut ppu::monochrome::MonochromePpu, tile_idx: usize, color: u8) {
    assert!(color <= 3);

    let offset = tile_idx * 16;
    let color_high = color >> 1;
    let color_low = color & 1;
    for i in 0..16 {
        if i % 2 == 0 {
            ppu.state.borrow_mut().tile_data[offset + i] = 0xff * color_low;
        } else {
            ppu.state.borrow_mut().tile_data[offset + i] = 0xff * color_high;
        }
    }
}

fn advance_frame(ppu: &mut ppu::monochrome::MonochromePpu) {
    for _ in 0..ppu::monochrome::FRAME_T_CYCLES {
        ppu.clock(None);
    }
}

#[test]
fn ppu_singlecolor() {
    let mut ppu = ppu::monochrome::MonochromePpu::new();

    ppu.state.borrow_mut().bg_map_1.fill(0);
    ppu.state.borrow_mut().lcdc = LCDC::LCD_ENABLE | LCDC::BG_ENABLE | LCDC::BG_TILE_DATA_AREA;
    ppu.state.borrow_mut().bgp = 0b11100100;

    for color in [0b00, 0b01, 0b10, 0b11] {
        println!("color: {:b}", color);
        set_tile_singlecolor(&mut ppu, 0, color);
        advance_frame(&mut ppu);
        let frame = ppu.get_frame();
        frame.pixels.iter().for_each(|&pix| {
            assert_eq!(
                pix,
                ppu::monochrome::color::calculate_monochrome_color(
                    ppu.state.borrow_mut().bgp,
                    color
                )
            )
        });
    }
}

#[test]
fn ppu_bgp() {
    let mut ppu = ppu::monochrome::MonochromePpu::new();

    ppu.state.borrow_mut().lcdc = LCDC::LCD_ENABLE | LCDC::BG_ENABLE | LCDC::BG_TILE_DATA_AREA;
    set_tile_singlecolor(&mut ppu, 0, 0b00);
    set_tile_singlecolor(&mut ppu, 1, 0b01);
    set_tile_singlecolor(&mut ppu, 2, 0b10);
    set_tile_singlecolor(&mut ppu, 3, 0b11);
    for i in 0..0x400 {
        ppu.state.borrow_mut().bg_map_1[i] = (i % 4) as u8;
    }

    for bgp in 0..=0xFF {
        ppu.state.borrow_mut().bgp = bgp;
        advance_frame(&mut ppu);

        let frame = ppu.get_frame();

        for color in 0..=3 {
            let i = color as usize * 8; // 8 pixel wide tiles
            assert_eq!(
                frame.pixels[i],
                ppu::monochrome::color::calculate_monochrome_color(bgp, color)
            );
        }
    }
}
