//! TODO: Docs
#![feature(asm)]

extern crate cgmath;

#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]

pub mod sys;
pub mod ray;
pub mod device;
pub mod scene;
pub mod buffer;
pub mod triangle_mesh;
//pub mod quad_mesh;
//pub mod instance;
//pub mod geometry;

pub use ray::{Ray, Hit, RayHit, IntersectContext};
pub use device::Device;
pub use scene::Scene;
pub use buffer::{Buffer, MappedBuffer};
pub use triangle_mesh::TriangleMesh;
//pub use quad_mesh::QuadMesh;
//pub use instance::Instance;
//pub use geometry::Geometry;

// Pull in some cleaned up enum and bitfield types directly,
// with prettier aliases
pub use sys::RTCFormat as Format;
pub use sys::RTCBuildQuality as BuildQuality;
pub use sys::RTCDeviceProperty as DeviceProperty;
pub use sys::RTCError as Error;
pub use sys::RTCBufferType as BufferType;
pub use sys::RTCGeometryType as GeometryType;
pub use sys::RTCSubdivisionMode as SubdivisionMode;

pub use sys::RTCIntersectContextFlags as IntersectContextFlags;
pub use sys::RTCCurveFlags as CurveFlags;
pub use sys::RTCSceneFlags as SceneFlags;
pub use sys::RTCBuildFlags as BuildFlags;

