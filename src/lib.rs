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

pub mod buffer;
mod callback;
pub mod device;
pub mod error;
mod geometry;
pub mod instance;
pub mod intersect_context;
pub mod ray;
pub mod ray_packet;
pub mod ray_stream;
pub mod scene;
pub mod soa_ray;
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
pub mod sys;
pub use buffer::{Buffer, BufferSize, BufferSlice, BufferView, BufferViewMut};
pub use device::{Config, Device, FrequencyLevel, Isa};
pub use instance::Instance;
pub use intersect_context::IntersectContext;
pub use ray::{Hit, Ray, RayHit};
pub use ray_packet::{Hit4, Ray4, RayHit4};
pub use ray_stream::{HitN, RayHitN, RayN};
pub use scene::Scene;
pub use soa_ray::{
    SoAHit, SoAHitIter, SoAHitIterMut, SoAHitRef, SoARay, SoARayIter, SoARayIterMut, SoARayRef,
    SoARayRefMut,
};

pub use geometry::*;

// Pull in some cleaned up enum and bitfield types directly,
// with prettier aliases
pub use sys::{
    RTCBufferType as BufferUsage, RTCBuildQuality as BuildQuality,
    RTCDeviceProperty as DeviceProperty, RTCError as Error, RTCFormat as Format,
    RTCGeometryType as GeometryType, RTCSubdivisionMode as SubdivisionMode,
};

pub use sys::{
    RTCBuildFlags as BuildFlags, RTCCurveFlags as CurveFlags,
    RTCIntersectContextFlags as IntersectContextFlags, RTCSceneFlags as SceneFlags,
};

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
