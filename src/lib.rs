//! TODO: Docs

use std::{mem, alloc};

extern crate cgmath;

pub mod buffer;
pub mod device;
pub mod geometry;
pub mod instance;
pub mod quad_mesh;
pub mod ray;
pub mod soa_ray;
pub mod ray_packet;
pub mod ray_stream;
pub mod scene;
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
pub mod sys;
pub mod triangle_mesh;

pub use buffer::{Buffer, MappedBuffer};
pub use device::Device;
pub use geometry::Geometry;
pub use instance::Instance;
pub use quad_mesh::QuadMesh;
pub use ray::{Ray, Hit, RayHit, IntersectContext};
pub use soa_ray::{SoARay, SoAHit, SoARayRef, SoARayRefMut,
                    SoARayIter, SoARayIterMut, SoAHitRef,
                    SoAHitIter, SoAHitIterMut};
pub use ray_packet::{Ray4, Hit4, RayHit4};
pub use ray_stream::{RayN, HitN, RayHitN};
pub use scene::{Scene, CommittedScene};
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
pub use sys::RTCSceneFlags as SceneFlags;

/// Utility for making specifically aligned vectors
pub fn aligned_vector<T>(len: usize, align: usize) -> Vec<T> {
    let t_size = mem::size_of::<T>();
    let t_align = mem::align_of::<T>();
    let layout =
        if t_align >= align {
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

