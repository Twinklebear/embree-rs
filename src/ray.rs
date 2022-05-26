use std::{f32, u32};

use crate::sys;

pub type Ray = sys::RTCRay;
pub type Hit = sys::RTCHit;
pub type RayHit = sys::RTCRayHit;

impl Ray {
    /// Create a new ray starting at `origin` and heading in direction `dir`
    pub fn new(origin: [f32; 3], dir: [f32; 3]) -> Ray {
        Ray::segment(origin, dir, 0.0, f32::INFINITY)
    }
    pub fn segment(origin: [f32; 3], dir: [f32; 3], tnear: f32, tfar: f32) -> Ray {
        sys::RTCRay {
            org_x: origin[0],
            org_y: origin[1],
            org_z: origin[2],
            dir_x: dir[0],
            dir_y: dir[1],
            dir_z: dir[2],
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
            ray,
            hit: Hit::new(),
        }
    }
}
