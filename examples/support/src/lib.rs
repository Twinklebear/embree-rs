extern crate glium;
extern crate image;
extern crate arcball;
extern crate cgmath;
extern crate clock_ticks;

pub mod vec3f;
pub mod camera;
pub mod display;

pub use vec3f::Vec3f;
pub use camera::Camera;
pub use display::Display;

/// Clamp `x` to be between `min` and `max`
pub fn clamp<T: PartialOrd>(x: T, min: T, max: T) -> T {
    if x < min { min } else if x > max { max } else { x }
}

