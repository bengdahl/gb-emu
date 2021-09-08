pub type RgbaColor = u32;

pub const COLOR_BLACK: RgbaColor = 0xFF000000;
pub const COLOR_DARKGRAY: RgbaColor = 0xFF777777;
pub const COLOR_LIGHTGRAY: RgbaColor = 0xFFAAAAAA;
pub const COLOR_WHITE: RgbaColor = 0xFFFFFFFF;

pub const COLORS: [RgbaColor; 4] = [COLOR_WHITE, COLOR_LIGHTGRAY, COLOR_DARKGRAY, COLOR_BLACK];

pub fn calculate_monochrome_color_id(palette: u8, pix: u8) -> usize {
    assert!(pix < 4);
    ((palette >> (pix * 2)) & 0x03) as usize
}
