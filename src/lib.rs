//! [![Crates.io](https://img.shields.io/crates/v/embree.svg)](https://crates.io/crates/embree)
//! [![Build Status](https://travis-ci.org/Twinklebear/embree-rs.svg?branch=master)](https://travis-ci.org/Twinklebear/embree-rs)
//!
//! Rust bindings to [Embree](http://embree.github.io/). These are still in
//! development, so a range of features are in progress.
//!
//! # Documentation
//!
//! Rust doc can be found [here](https://docs.rs/embree/).
//! Embree documentation can be found [here](https://embree.github.io/api.html).
//! See the [examples/](https://github.com/Twinklebear/embree-rs/tree/master/examples)
//! for some example applications using the bindings.

use std::{alloc, mem};

pub mod bezier_curve;
pub mod bspline_curve;
pub mod buffer;
pub mod catmull_rom_curve;
pub mod curve;
pub mod device;
pub mod geometry;
pub mod hermite_curve;
pub mod instance;
pub mod linear_curve;
pub mod quad_mesh;
pub mod ray;
pub mod ray_packet;
pub mod ray_stream;
pub mod scene;
pub mod soa_ray;
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
pub mod sys;
pub mod triangle_mesh;
mod callback;

pub use bezier_curve::BezierCurve;
pub use bspline_curve::BsplineCurve;
pub use buffer::{Buffer, MappedBuffer};
pub use catmull_rom_curve::CatmullRomCurve;
pub use curve::CurveType;
pub use device::{Config, Device, FrequencyLevel, Isa};
pub use geometry::Geometry;
pub use hermite_curve::HermiteCurve;
pub use instance::Instance;
pub use linear_curve::LinearCurve;
pub use quad_mesh::QuadMesh;
pub use ray::{Hit, IntersectContext, Ray, RayHit};
pub use ray_packet::{Hit4, Ray4, RayHit4};
pub use ray_stream::{HitN, RayHitN, RayN};
pub use scene::{Scene, SceneFlags};
pub use soa_ray::{
    SoAHit, SoAHitIter, SoAHitIterMut, SoAHitRef, SoARay, SoARayIter, SoARayIterMut, SoARayRef,
    SoARayRefMut,
};
pub use triangle_mesh::TriangleMesh;

// Pull in some cleaned up enum and bitfield types directly,
// with prettier aliases
pub use sys::RTCBufferType as BufferType;
pub use sys::RTCBuildQuality as BuildQuality;
pub use sys::RTCDeviceProperty as DeviceProperty;
pub use sys::RTCError as Error;
pub use sys::RTCFormat as Format;
pub use sys::RTCGeometryType as GeometryType;
pub use sys::RTCSubdivisionMode as SubdivisionMode;

pub use sys::RTCBuildFlags as BuildFlags;
pub use sys::RTCCurveFlags as CurveFlags;
pub use sys::RTCIntersectContextFlags as IntersectContextFlags;

/// Utility for making specifically aligned vectors
pub fn aligned_vector<T>(len: usize, align: usize) -> Vec<T> {
    let t_size = mem::size_of::<T>();
    let t_align = mem::align_of::<T>();
    let layout = if t_align >= align {
        alloc::Layout::from_size_align(t_size * len, t_align).unwrap()
    } else {
        alloc::Layout::from_size_align(t_size * len, align).unwrap()
    };
    unsafe {
        let mem = alloc::alloc(layout);
        assert_eq!((mem as usize) % 16, 0);
        Vec::<T>::from_raw_parts(mem as *mut T, len, len)
    }
}
pub fn aligned_vector_init<T: Copy>(len: usize, align: usize, init: T) -> Vec<T> {
    let mut v = aligned_vector::<T>(len, align);
    for x in v.iter_mut() {
        *x = init;
    }
    v
}

#[test]
fn test_aligned_vector_alloc() {
    let v = aligned_vector_init::<f32>(24, 16, 1.0);
    for x in v.iter() {
        assert_eq!(*x, 1.0);
    }
}
