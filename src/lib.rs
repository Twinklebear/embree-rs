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

mod buffer;
mod callback;
mod device;
mod error;
mod geometry;
mod intersect_context;
mod ray;
mod scene;

/// Automatically generated bindings to the Embree C API.
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
pub mod sys;

pub use buffer::*;
pub use device::*;
pub use error::*;
pub use geometry::*;
pub use intersect_context::*;
pub use ray::*;
pub use scene::*;

// Pull in some cleaned up enum and bitfield types directly,
// with prettier aliases
pub type Bounds = sys::RTCBounds;

/// Defines the type of slots to assign data buffers to.
///
/// For most geometry types the [`BufferUsage::INDEX`] slot is used to assign
/// an index buffer, while the [`BufferUsage::VERTEX`] is used to assign the
/// corresponding vertex buffer.
///
/// The [`BufferUsage::VERTEX_ATTRIBUTE`] slot can get used to assign
/// arbitrary additional vertex data which can get interpolated using the
/// [`rtcInterpolate`] API call.
///
/// The [`BufferUsage::NORMAL`], [`BufferUsage::TANGENT`], and
/// [`BufferUsage::NORMAL_DERIVATIVE`] are special buffers required to assign
/// per vertex normals, tangents, and normal derivatives for some curve types.
///
/// The [`BufferUsage::GRID`] buffer is used to assign the grid primitive buffer
/// for grid geometries (see [`GeometryKind::GRID`]).
///
/// The [`BufferUsage::FACE`], [`BufferUsage::LEVEL`],
/// [`BufferUsage::EDGE_CREASE_INDEX`], [`BufferUsage::EDGE_CREASE_WEIGHT`],
/// [`BufferUsage::VERTEX_CREASE_INDEX`], [`BufferUsage::VERTEX_CREASE_WEIGHT`],
/// and [`BufferUsage::HOLE`] are special buffers required to create subdivision
/// meshes (see [`GeometryKind::SUBDIVISION`]).
///
/// [`BufferUsage::FLAGS`] can get used to add additional flag per primitive of
/// a geometry, and is currently only used for linear curves.
pub type BufferUsage = sys::RTCBufferType;
pub type BuildQuality = sys::RTCBuildQuality;
pub type BuildFlags = sys::RTCBuildFlags;
pub type CurveFlags = sys::RTCCurveFlags;
pub type DeviceProperty = sys::RTCDeviceProperty;
pub type Error = sys::RTCError;
pub type Format = sys::RTCFormat;
pub type IntersectContextFlags = sys::RTCIntersectContextFlags;
pub type SceneFlags = sys::RTCSceneFlags;
pub type SubdivisionMode = sys::RTCSubdivisionMode;
/// The type of a geometry, used to determine which geometry type to create.
pub type GeometryKind = sys::RTCGeometryType;
pub type QuaternionDecomposition = sys::RTCQuaternionDecomposition;

/// The invalid ID for Embree intersection results (e.g. `Hit::geomID`,
/// `Hit::primID`, etc.)
pub const INVALID_ID: u32 = u32::MAX;

impl Default for Bounds {
    fn default() -> Self {
        Bounds {
            lower_x: f32::INFINITY,
            lower_y: f32::INFINITY,
            lower_z: f32::INFINITY,
            align0: 0.0,
            upper_x: f32::INFINITY,
            upper_y: f32::INFINITY,
            upper_z: f32::INFINITY,
            align1: 0.0,
        }
    }
}

impl Bounds {
    /// Returns the lower bounds of the bounding box.
    pub fn lower(&self) -> [f32; 3] { [self.lower_x, self.lower_y, self.lower_z] }

    /// Returns the upper bounds of the bounding box.
    pub fn upper(&self) -> [f32; 3] { [self.upper_x, self.upper_y, self.upper_z] }
}

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
