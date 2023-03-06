pub mod camera;
mod common;
pub mod display;

pub use common::*;

pub use camera::Camera;
pub use display::Display;

pub use image::{Rgba, RgbaImage};
pub use rayon::{iter::*, prelude::*, slice::*, vec::*};
pub mod math {
    pub use cgmath::*;
}

/// The type of ray used for rendering with Embree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    /// A single ray.
    Normal,
    /// A stream of rays.
    Stream,
}

/// An image that is tiled into smaller tiles for parallel rendering.
///
/// Tiles and pixels inside tiles are stored in a flat array in row-major order.
pub struct TiledImage {
    pub width: u32,
    pub height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub tile_size: u32,
    pub num_tiles_x: u32,
    pub num_tiles_y: u32,
    pub num_tiles: u32,
    pub pixels: Vec<u32>,
}

impl TiledImage {
    /// Create a new tiled image.
    pub fn new(width: u32, height: u32, tile_width: u32, tile_height: u32) -> Self {
        let num_tiles_x = (width + tile_width - 1) / tile_width;
        let num_tiles_y = (height + tile_height - 1) / tile_height;
        let tile_size = tile_width * tile_height;
        let num_tiles = num_tiles_x * num_tiles_y;
        Self {
            width,
            height,
            tile_width,
            tile_height,
            tile_size,
            num_tiles_x,
            num_tiles_y,
            num_tiles,
            pixels: vec![0; (num_tiles * tile_size) as usize],
        }
    }

    /// Write the tiled image to a flat image.
    pub fn write_to_image(&self, image: &mut RgbaImage) {
        for j in 0..self.height {
            for i in 0..self.width {
                let tile_x = i / self.tile_width;
                let tile_y = j / self.tile_height;
                let tile_index = tile_y * self.num_tiles_x + tile_x;
                let tile_offset = (tile_index * self.tile_size) as usize;
                let tile_i = i % self.tile_width;
                let tile_j = j % self.tile_height;
                let tile_pixel_index = tile_offset + (tile_j * self.tile_width + tile_i) as usize;
                let pixel = self.pixels[tile_pixel_index];
                image.put_pixel(i, j, Rgba(u32_to_rgba(pixel)));
            }
        }
    }

    pub fn tile_mut(&mut self, index: usize) -> Tile<'_> {
        let idx = index as u32;
        let x = (idx % self.num_tiles_x) * self.tile_width;
        let y = (idx / self.num_tiles_x) * self.tile_height;
        let offset = (idx * self.tile_size) as usize;
        Tile {
            idx,
            x,
            y,
            w: self.tile_width,
            h: self.tile_height,
            pixels: &mut self.pixels[offset..offset + self.tile_size as usize],
        }
    }

    pub fn tiles_mut(&mut self) -> impl Iterator<Item = Tile<'_>> {
        self.pixels
            .chunks_mut(self.tile_size as usize)
            .enumerate()
            .map(|(i, pixels)| {
                let idx = i as u32;
                let x = (idx % self.num_tiles_x) * self.tile_width;
                let y = (idx / self.num_tiles_x) * self.tile_height;
                Tile {
                    idx,
                    x,
                    y,
                    w: self.tile_width,
                    h: self.tile_height,
                    pixels,
                }
            })
    }

    /// Iterate over the tiles of the tiled image.
    pub fn par_tiles_mut(&mut self) -> impl IndexedParallelIterator<Item = Tile<'_>> {
        self.pixels
            .par_chunks_mut(self.tile_size as usize)
            .enumerate()
            .map(|(i, pixels)| {
                let idx = i as u32;
                let x = (idx % self.num_tiles_x) * self.tile_width;
                let y = (idx / self.num_tiles_x) * self.tile_height;
                Tile {
                    idx,
                    x,
                    y,
                    w: self.tile_width,
                    h: self.tile_height,
                    pixels,
                }
            })
    }

    /// Reset the pixels of the tiled image.
    pub fn reset_pixels(&mut self) {
        unsafe {
            std::ptr::write_bytes(self.pixels.as_mut_ptr(), 0, self.pixels.len());
        }
    }
}

/// A tile of the tiled image.
pub struct Tile<'a> {
    /// The index of the tile.
    pub idx: u32,
    /// The x coordinate of the tile in the image.
    pub x: u32,
    /// The y coordinate of the tile in the image.
    pub y: u32,
    /// The width of the tile.
    pub w: u32,
    /// The height of the tile.
    pub h: u32,
    /// The pixels of the tile.
    pub pixels: &'a mut [u32],
}

/// Convert a u32 to a RGBA color.
#[inline(always)]
pub const fn u32_to_rgba(val: u32) -> [u8; 4] {
    let r = (val >> 24) as u8;
    let g = (val >> 16) as u8;
    let b = (val >> 8) as u8;
    let a = val as u8;
    [r, g, b, a]
}

/// Convert a RGBA color to a u32.
#[inline(always)]
pub const fn rgba_to_u32(r: u8, g: u8, b: u8, a: u8) -> u32 {
    ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32)
}

/// Clamp `x` to be between `min` and `max`
pub fn clamp<T: PartialOrd>(x: T, min: T, max: T) -> T {
    if x < min {
        min
    } else if x > max {
        max
    } else {
        x
    }
}
