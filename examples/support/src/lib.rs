extern crate arcball;
extern crate cgmath;
extern crate clock_ticks;
extern crate futures;
extern crate image;

type Mat4 = cgmath::Matrix4<f32>;
type CgPoint = cgmath::Point3<f32>;
type CgVec = cgmath::Vector3<f32>;
type Vector2 = cgmath::Vector2<f32>;
type Vector3 = cgmath::Vector3<f32>;
type Vector4 = cgmath::Vector4<f32>;

pub mod camera;
mod common;
pub mod display;

pub use common::*;

pub use camera::Camera;
pub use display::Display;

pub use image::{Rgba, RgbaImage};

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
