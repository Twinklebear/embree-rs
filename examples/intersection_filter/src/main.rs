//! This example shows how to use filter callback functions to efficiently
//! implement transparent objects.
//!
//! The filter function is used for primary rays lets the ray pass through
//! the geometry if it is entirely transparent. Otherwise, the shading loop
//! handles the transparency properly, by potentially shooting secondary rays.
//!
//! The filter function used for shadow rays accumulates the transparency of
//! all surfaces along the ray, and terminates traversal if an opaque surface
//! occluder is hit.

use embree::Ray;
use glam::Vec3;

const HIT_LIST_LEN: usize = 16;

// Extended ray structure that includes total transparency along the ray.
struct Ray2 {
    ray: Ray,
    transparency: f32, // accumulated transparency
    first_hit: u32,    // index of first hit
    last_hit: u32,     // index of last hit
    hit_geom_ids: [u32; HIT_LIST_LEN],
    hit_prim_ids: [u32; HIT_LIST_LEN],
}

fn transparency_function(h: Vec3) -> f32 {
    let v = ((4.0 * h.x).sin() * (4.0 * h.y).cos() * (4.0 * h.z).sin()).abs();
    ((v - 0.1) * 3.0).clamp(0.0, 1.0)
}

fn main() {
    println!("Hello, world!");
}
