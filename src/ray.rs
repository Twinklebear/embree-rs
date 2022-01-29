use cgmath::Vector3;
use std::{f32, u32};

use crate::sys;

pub type Ray = sys::RTCRay;
pub type Hit = sys::RTCHit;
pub type RayHit = sys::RTCRayHit;
pub type IntersectContext = sys::RTCIntersectContext;

impl Ray {
    /// Create a new ray starting at `origin` and heading in direction `dir`
    pub fn new(origin: Vector3<f32>, dir: Vector3<f32>) -> Ray {
        Ray::segment(origin, dir, 0.0, f32::INFINITY)
    }
    pub fn segment(origin: Vector3<f32>, dir: Vector3<f32>, tnear: f32, tfar: f32) -> Ray {
        sys::RTCRay {
            org_x: origin.x,
            org_y: origin.y,
            org_z: origin.z,
            dir_x: dir.x,
            dir_y: dir.y,
            dir_z: dir.z,
            tnear: tnear,
            tfar: tfar,
            time: 0.0,
            mask: u32::MAX,
            id: 0,
            flags: 0,
        }
    }
}

impl Hit {
    pub fn new() -> Hit {
        sys::RTCHit {
            Ng_x: 0.0,
            Ng_y: 0.0,
            Ng_z: 0.0,
            u: 0.0,
            v: 0.0,
            primID: u32::MAX,
            geomID: u32::MAX,
            instID: [u32::MAX; 1],
        }
    }
    pub fn hit(&self) -> bool {
        self.geomID != u32::MAX
    }
}

impl RayHit {
    pub fn new(ray: Ray) -> RayHit {
        sys::RTCRayHit {
            ray: ray,
            hit: Hit::new(),
        }
    }
}

impl IntersectContext {
    pub fn coherent() -> IntersectContext {
        IntersectContext::new(sys::RTCIntersectContextFlags::COHERENT)
    }
    pub fn incoherent() -> IntersectContext {
        IntersectContext::new(sys::RTCIntersectContextFlags::INCOHERENT)
    }
    fn new(flags: sys::RTCIntersectContextFlags) -> IntersectContext {
        sys::RTCIntersectContext {
            flags: flags,
            filter: None,
            instID: [u32::MAX; 1],
        }
    }
}
