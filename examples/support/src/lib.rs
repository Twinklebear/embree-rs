pub mod camera;
mod common;
pub mod display;

pub use common::*;

pub use camera::Camera;
pub use display::Display;
pub use egui;

use embree::Scene;
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

/// Shading mode for the tutorial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShadingMode {
    /// Default tutorial shader
    Default,
    /// EyeLight shading
    EyeLight,
    /// Occlusion shading; only traces occlusion rays
    Occlusion,
    /// UV debug shading
    UV,
    /// Texture coordinates debug shading
    TexCoords,
    /// Grid texture debug shading
    TexCoordsGrid,
    /// Visualisation of shading normals
    Normal,
    /// CPU cycles visualisation
    CPUCycles,
    /// Visualisation of geometry IDs
    GeometryID,
    /// Visualisation of geometry and primitive IDs
    GeometryPrimitiveID,
    /// Ambient occlusion shading
    AmbientOcclusion,
}

/// An image that is tiled into smaller tiles for parallel rendering.
///
/// Tiles and pixels inside tiles are stored in a flat array in row-major order.
/// The pixel is encoded as a 4-byte RGBA value.
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
    /// Whether the image is being reinterpreted as a non-tiled image.
    is_tiled: bool,
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
            is_tiled: true,
        }
    }

    pub fn reinterpret_as_none_tiled(&mut self) { self.is_tiled = false; }

    pub fn reinterpret_as_tiled(&mut self) { self.is_tiled = true; }

    /// Write the tiled image to a flat image.
    pub fn write_to_image(&self, image: &mut RgbaImage) {
        if !self.is_tiled {
            for i in 0..self.width {
                for j in 0..self.height {
                    let pixel = self.pixels[(j * self.width + i) as usize];
                    image.put_pixel(i, j, Rgba(pixel.to_le_bytes()));
                }
            }
        } else {
            for j in 0..self.height {
                for i in 0..self.width {
                    let tile_x = i / self.tile_width;
                    let tile_y = j / self.tile_height;
                    let tile_index = tile_y * self.num_tiles_x + tile_x;
                    let tile_offset = (tile_index * self.tile_size) as usize;
                    let tile_i = i % self.tile_width;
                    let tile_j = j % self.tile_height;
                    let tile_pixel_index =
                        tile_offset + (tile_j * self.tile_width + tile_i) as usize;
                    let pixel = self.pixels[tile_pixel_index];
                    image.put_pixel(i, j, Rgba(pixel.to_le_bytes()));
                }
            }
        }
    }

    /// Write the tiled image to a flat image buffer.
    pub fn write_to_flat_buffer(&self, buffer: &mut [u8]) {
        debug_assert!(buffer.len() >= (self.width * self.height * 4) as usize);
        if !self.is_tiled {
            unsafe {
                buffer.as_mut_ptr().copy_from_nonoverlapping(
                    self.pixels.as_ptr() as *const u8,
                    (self.width * self.height * 4) as usize,
                );
            }
        } else {
            for tile in self.tiles() {
                let base_offset = (tile.y * self.width + tile.x) as usize * 4;
                // Copy the tile pixels to the buffer per row.
                for i in 0..self.tile_height {
                    let row_offset = self.width as usize * 4 * i as usize;
                    unsafe {
                        buffer
                            .as_mut_ptr()
                            .add(base_offset + row_offset)
                            .copy_from_nonoverlapping(
                                tile.pixels.as_ptr().add((i * self.tile_width) as usize)
                                    as *const u8,
                                self.tile_width as usize * 4,
                            );
                    }
                }
            }
        }
    }

    pub fn tile_mut(&mut self, index: usize) -> TileMut<'_> {
        debug_assert!(self.is_tiled);
        let idx = index as u32;
        let x = (idx % self.num_tiles_x) * self.tile_width;
        let y = (idx / self.num_tiles_x) * self.tile_height;
        let offset = (idx * self.tile_size) as usize;
        TileMut {
            idx,
            x,
            y,
            w: self.tile_width,
            h: self.tile_height,
            pixels: &mut self.pixels[offset..offset + self.tile_size as usize],
        }
    }

    pub fn tile(&self, index: usize) -> Tile<'_> {
        debug_assert!(self.is_tiled);
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
            pixels: &self.pixels[offset..offset + self.tile_size as usize],
        }
    }

    pub fn tiles(&self) -> impl Iterator<Item = Tile<'_>> {
        debug_assert!(self.is_tiled);
        self.pixels
            .chunks(self.tile_size as usize)
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

    pub fn tiles_mut(&mut self) -> impl Iterator<Item = TileMut<'_>> {
        debug_assert!(self.is_tiled);
        self.pixels
            .chunks_mut(self.tile_size as usize)
            .enumerate()
            .map(|(i, pixels)| {
                let idx = i as u32;
                let x = (idx % self.num_tiles_x) * self.tile_width;
                let y = (idx / self.num_tiles_x) * self.tile_height;
                TileMut {
                    idx,
                    x,
                    y,
                    w: self.tile_width,
                    h: self.tile_height,
                    pixels,
                }
            })
    }

    pub fn par_tiles(&self) -> impl IndexedParallelIterator<Item = Tile<'_>> {
        debug_assert!(self.is_tiled);
        self.pixels
            .par_chunks(self.tile_size as usize)
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
    pub fn par_tiles_mut(&mut self) -> impl IndexedParallelIterator<Item = TileMut<'_>> {
        debug_assert!(self.is_tiled);
        self.pixels
            .par_chunks_mut(self.tile_size as usize)
            .enumerate()
            .map(|(i, pixels)| {
                let idx = i as u32;
                let x = (idx % self.num_tiles_x) * self.tile_width;
                let y = (idx / self.num_tiles_x) * self.tile_height;
                TileMut {
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
    /// The index of the tile in the tiled image.
    pub idx: u32,
    /// The x coordinate of the tile in the image.
    pub x: u32,
    /// The y coordinate of the tile in the image.
    pub y: u32,
    /// The width of the tile.
    pub w: u32,
    /// The height of the tile.
    pub h: u32,
    /// The pixels of the tile, in RGBA format.
    pub pixels: &'a [u32],
}

/// A mutable tile of the tiled image.
pub struct TileMut<'a> {
    /// The index of the tile in the tiled image.
    pub idx: u32,
    /// The x coordinate of the tile in the image.
    pub x: u32,
    /// The y coordinate of the tile in the image.
    pub y: u32,
    /// The width of the tile.
    pub w: u32,
    /// The height of the tile.
    pub h: u32,
    /// The pixels of the tile, in RGBA format.
    pub pixels: &'a mut [u32],
}

/// Convert a RGBA color to a u32 in the format 0xAABBGGRR.
#[inline(always)]
pub const fn rgba_to_u32(r: u8, g: u8, b: u8, a: u8) -> u32 {
    ((a as u32) << 24) | ((b as u32) << 16) | ((g as u32) << 8) | (r as u32)
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

#[derive(Clone, Debug)]
pub struct DebugState<T: Sized> {
    pub scene: Scene<'static>,
    pub user: T,
}

unsafe impl<T> Send for DebugState<T> where T: Sized + Send + Sync {}
unsafe impl<T> Sync for DebugState<T> where T: Sized + Send + Sync {}
