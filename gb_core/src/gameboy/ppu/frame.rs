use std::ops::{Index, IndexMut};

use super::color::RgbaColor;

#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pixels: [RgbaColor; 144 * 160],
}

impl Frame {
    pub fn new() -> Self {
        Self {
            pixels: [0; 144 * 160],
        }
    }

    pub fn rows(&self) -> impl Iterator<Item = &[RgbaColor; 160]> {
        self.pixels.array_chunks::<160>()
    }

    pub fn iter(&self) -> impl Iterator<Item = &RgbaColor> {
        self.pixels.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut RgbaColor> {
        self.pixels.iter_mut()
    }
}

impl Default for Frame {
    fn default() -> Self {
        Self::new()
    }
}

fn assert_coords_in_range(x: usize, y: usize) {
    assert!(y < 144, "y ({}) is out of range (<144)", y);
    assert!(x < 160, "x ({}) is out of range (<160)", x);
}

impl Index<(usize, usize)> for Frame {
    type Output = RgbaColor;
    fn index(&self, (x, y): (usize, usize)) -> &Self::Output {
        assert_coords_in_range(x, y);
        &self.pixels[y * 160 + x]
    }
}

impl IndexMut<(usize, usize)> for Frame {
    fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut Self::Output {
        assert_coords_in_range(x, y);
        &mut self.pixels[y * 160 + x]
    }
}

impl Index<usize> for Frame {
    type Output = RgbaColor;
    fn index(&self, i: usize) -> &Self::Output {
        &self.pixels[i]
    }
}

impl IndexMut<usize> for Frame {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.pixels[index]
    }
}
