extern crate arcball;
extern crate cgmath;
extern crate clock_ticks;
extern crate glium;
extern crate image;

type Mat4 = cgmath::Matrix4<f32>;
type CgPoint = cgmath::Point3<f32>;
type CgVec = cgmath::Vector3<f32>;
type Vector2 = cgmath::Vector2<f32>;
type Vector3 = cgmath::Vector3<f32>;
type Vector4 = cgmath::Vector4<f32>;

pub mod aabb;
pub mod camera;
pub mod display;

pub use aabb::AABB;
pub use camera::Camera;
pub use display::Display;

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

fn vec_min(v1: &Vector3, v2: &Vector3) -> Vector3 {
    Vector3::new(v1.x.min(v2.x), v1.y.min(v2.y), v1.z.min(v2.z))
}

fn vec_max(v1: &Vector3, v2: &Vector3) -> Vector3 {
    Vector3::new(v1.x.max(v2.x), v1.y.max(v2.y), v1.z.max(v2.z))
}
