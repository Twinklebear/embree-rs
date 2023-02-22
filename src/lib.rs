#![feature(portable_simd)]
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

use std::{alloc, mem, mem::MaybeUninit};

mod buffer;
mod bvh;
mod callback;
mod context;
mod device;
mod error;
mod geometry;
mod ray;
mod scene;

/// Automatically generated bindings to the Embree C API.
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
pub mod sys;

pub use buffer::*;
pub use bvh::*;
pub use context::*;
pub use device::*;
pub use error::*;
pub use geometry::*;
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

/// Structure that represents a quaternion decomposition of an affine
/// transformation.
///
/// The affine transformation can be decomposed into three parts:
///
/// 1. A upper triangular scaling/skew/shift matrix
///
///   ```
///   | scale_x  skew_xy  skew_xz  shift_x |
///   |   0      scale_y  skew_yz  shift_y |
///   |   0         0     scale_z  shitf_z |
///   |   0         0        0         1   |
///   ```
///
/// 2. A translation matrix
///   ```
///   | 1   0   0 translation_x |
///   | 0   1   0 translation_y |
///   | 0   0   1 translation_z |
///   | 0   0   0       1       |
///   ```
///
/// 3. A rotation matrix R, represented as a quaternion
///   ```quaternion_r + i * quaternion_i + j * quaternion_j + k *
/// quaternion_k```   where i, j, k are the imaginary unit vectors. The passed
/// quaternion will   be normalized internally.
///
/// The affine transformation matrix corresponding to a quaternion decomposition
/// is TRS and a point `p = (x, y, z, 1)^T` is transformed as follows:
///
/// ```
/// p' = T * R * S * p
/// ```
pub type QuaternionDecomposition = sys::RTCQuaternionDecomposition;

impl Default for QuaternionDecomposition {
    fn default() -> Self { QuaternionDecomposition::identity() }
}

impl QuaternionDecomposition {
    /// Create a new quaternion decomposition with the identity transformation.
    pub fn identity() -> Self {
        QuaternionDecomposition {
            scale_x: 1.0,
            scale_y: 1.0,
            scale_z: 1.0,
            skew_xy: 0.0,
            skew_xz: 0.0,
            skew_yz: 0.0,
            shift_x: 0.0,
            shift_y: 0.0,
            shift_z: 0.0,
            quaternion_r: 1.0,
            quaternion_i: 0.0,
            quaternion_j: 0.0,
            quaternion_k: 0.0,
            translation_x: 0.0,
            translation_y: 0.0,
            translation_z: 0.0,
        }
    }

    /// Returns the scale part of the decomposition.
    pub fn scale(&self) -> [f32; 3] { [self.scale_x, self.scale_y, self.scale_z] }

    /// Returns the skew part of the decomposition.
    pub fn skew(&self) -> [f32; 3] { [self.skew_xy, self.skew_xz, self.skew_yz] }

    /// Returns the shift part of the decomposition.
    pub fn shift(&self) -> [f32; 3] { [self.shift_x, self.shift_y, self.shift_z] }

    /// Returns the translation part of the decomposition.
    pub fn quaternion(&self) -> [f32; 4] {
        [
            self.quaternion_r,
            self.quaternion_i,
            self.quaternion_j,
            self.quaternion_k,
        ]
    }

    /// Set the quaternion part of the decomposition.
    pub fn set_quaternion(&mut self, quaternion: [f32; 4]) {
        self.quaternion_r = quaternion[0];
        self.quaternion_i = quaternion[1];
        self.quaternion_j = quaternion[2];
        self.quaternion_k = quaternion[3];
    }

    /// Set the scaling part of the decomposition.
    pub fn set_scale(&mut self, scale: [f32; 3]) {
        self.scale_x = scale[0];
        self.scale_y = scale[1];
        self.scale_z = scale[2];
    }

    /// Set the skew part of the decomposition.
    pub fn set_skew(&mut self, skew: [f32; 3]) {
        self.skew_xy = skew[0];
        self.skew_xz = skew[1];
        self.skew_yz = skew[2];
    }

    /// Set the shift part of the decomposition.
    pub fn set_shift(&mut self, shift: [f32; 3]) {
        self.shift_x = shift[0];
        self.shift_y = shift[1];
        self.shift_z = shift[2];
    }

    /// Set the translation part of the decomposition.
    pub fn set_translation(&mut self, translation: [f32; 3]) {
        self.translation_x = translation[0];
        self.translation_y = translation[1];
        self.translation_z = translation[2];
    }
}

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

/// Object used to traverses the BVH and calls a user defined callback function
/// for each primitive of the scene that intersects the query domain.
///
/// See [`Scene::point_query`] for more information.
pub type PointQuery = sys::RTCPointQuery;

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
