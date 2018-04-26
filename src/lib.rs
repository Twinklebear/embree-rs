//! TODO: Docs
#![feature(attr_literals)]

extern crate cgmath;

pub mod buffer;
pub mod device;
pub mod geometry;
pub mod instance;
pub mod quad_mesh;
pub mod ray;
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
pub use ray::{Hit, IntersectContext, Ray, RayHit};
pub use scene::Scene;
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
