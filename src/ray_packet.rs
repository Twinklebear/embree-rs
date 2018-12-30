use cgmath::Vector3;
use std::{f32, u32};
use std::iter::Iterator;

use sys;

pub type Ray4 = sys::RTCRay4;
pub type Hit4 = sys::RTCHit4;
pub type RayHit4 = sys::RTCRayHit4;

impl Ray4 {
    pub fn new(origin: [Vector3<f32>; 4], dir: [Vector3<f32>; 4]) -> Ray4 {
        Ray4::segment(origin, dir, [0.0; 4], [f32::INFINITY; 4])
    }
    pub fn segment(origin: [Vector3<f32>; 4], dir: [Vector3<f32>; 4],
                   tnear: [f32; 4], tfar: [f32; 4]) -> Ray4 {
        sys::RTCRay4 {
            org_x: [origin[0].x, origin[1].x, origin[2].x, origin[3].x],
            org_y: [origin[0].y, origin[1].y, origin[2].y, origin[3].y],
            org_z: [origin[0].y, origin[1].y, origin[2].y, origin[3].y],
            dir_x: [dir[0].x, dir[1].x, dir[2].x, dir[3].x],
            dir_y: [dir[0].y, dir[1].y, dir[2].y, dir[3].y],
            dir_z: [dir[0].y, dir[1].y, dir[2].y, dir[3].y],
            tnear: tnear,
            tfar: tfar,
            time: [0.0; 4],
            mask: [u32::MAX; 4],
            id: [0; 4],
            flags: [0; 4],
        }
    }
}

impl Hit4 {
    pub fn new() -> Hit4 {
        sys::RTCHit4 {
            Ng_x: [0.0; 4],
            Ng_y: [0.0; 4],
            Ng_z: [0.0; 4],
            u: [0.0; 4],
            v: [0.0; 4],
            primID: [u32::MAX; 4],
            geomID: [u32::MAX; 4],
            instID: [[u32::MAX; 4]],
        }
    }
    pub fn any_hit(&self) -> bool {
        self.hits().fold(false, |acc, g| acc || g)
    }
    pub fn hits<'a>(&'a self) -> impl Iterator<Item=bool> + 'a {
        self.geomID.iter().map(|g| *g != u32::MAX)
    }
}

impl RayHit4 {
    pub fn new(ray: Ray4) -> RayHit4 {
        sys::RTCRayHit4 {
            ray: ray,
            hit: Hit4::new(),
        }
    }
}

