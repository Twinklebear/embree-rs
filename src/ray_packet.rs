use cgmath::Vector3;
use std::{f32, u32};

use crate::soa_ray::{SoAHit, SoAHitIter, SoAHitRef, SoARay, SoARayIter, SoARayIterMut};
use crate::sys;

pub type Ray4 = sys::RTCRay4;
pub type Hit4 = sys::RTCHit4;
pub type RayHit4 = sys::RTCRayHit4;

impl Ray4 {
    pub fn empty() -> Ray4 {
        Ray4::segment(
            [Vector3::new(0.0, 0.0, 0.0); 4],
            [Vector3::new(0.0, 0.0, 0.0); 4],
            [0.0; 4],
            [f32::INFINITY; 4],
        )
    }
    pub fn new(origin: [Vector3<f32>; 4], dir: [Vector3<f32>; 4]) -> Ray4 {
        Ray4::segment(origin, dir, [0.0; 4], [f32::INFINITY; 4])
    }
    pub fn segment(
        origin: [Vector3<f32>; 4],
        dir: [Vector3<f32>; 4],
        tnear: [f32; 4],
        tfar: [f32; 4],
    ) -> Ray4 {
        sys::RTCRay4 {
            org_x: [origin[0].x, origin[1].x, origin[2].x, origin[3].x],
            org_y: [origin[0].y, origin[1].y, origin[2].y, origin[3].y],
            org_z: [origin[0].z, origin[1].z, origin[2].z, origin[3].z],
            dir_x: [dir[0].x, dir[1].x, dir[2].x, dir[3].x],
            dir_y: [dir[0].y, dir[1].y, dir[2].y, dir[3].y],
            dir_z: [dir[0].z, dir[1].z, dir[2].z, dir[3].z],
            tnear: tnear,
            tfar: tfar,
            time: [0.0; 4],
            mask: [u32::MAX; 4],
            id: [0; 4],
            flags: [0; 4],
        }
    }
    pub fn iter(&self) -> SoARayIter<Ray4> {
        SoARayIter::new(self, 4)
    }
    pub fn iter_mut(&mut self) -> SoARayIterMut<Ray4> {
        SoARayIterMut::new(self, 4)
    }
}

impl SoARay for Ray4 {
    fn org(&self, i: usize) -> Vector3<f32> {
        Vector3::new(self.org_x[i], self.org_y[i], self.org_z[i])
    }
    fn set_org(&mut self, i: usize, o: Vector3<f32>) {
        self.org_x[i] = o.x;
        self.org_y[i] = o.y;
        self.org_z[i] = o.z;
    }

    fn dir(&self, i: usize) -> Vector3<f32> {
        Vector3::new(self.dir_x[i], self.dir_y[i], self.dir_z[i])
    }
    fn set_dir(&mut self, i: usize, d: Vector3<f32>) {
        self.dir_x[i] = d.x;
        self.dir_y[i] = d.y;
        self.dir_z[i] = d.z;
    }

    fn tnear(&self, i: usize) -> f32 {
        self.tnear[i]
    }
    fn set_tnear(&mut self, i: usize, near: f32) {
        self.tnear[i] = near;
    }

    fn tfar(&self, i: usize) -> f32 {
        self.tfar[i]
    }
    fn set_tfar(&mut self, i: usize, far: f32) {
        self.tfar[i] = far;
    }

    fn time(&self, i: usize) -> f32 {
        self.time[i]
    }
    fn set_time(&mut self, i: usize, time: f32) {
        self.time[i] = time;
    }

    fn mask(&self, i: usize) -> u32 {
        self.mask[i]
    }
    fn set_mask(&mut self, i: usize, mask: u32) {
        self.mask[i] = mask;
    }

    fn id(&self, i: usize) -> u32 {
        self.id[i]
    }
    fn set_id(&mut self, i: usize, id: u32) {
        self.id[i] = id;
    }

    fn flags(&self, i: usize) -> u32 {
        self.flags[i]
    }
    fn set_flags(&mut self, i: usize, flags: u32) {
        self.flags[i] = flags;
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
    pub fn hits<'a>(&'a self) -> impl Iterator<Item = bool> + 'a {
        self.geomID.iter().map(|g| *g != u32::MAX)
    }
    pub fn iter(&self) -> SoAHitIter<Hit4> {
        SoAHitIter::new(self, 4)
    }
    pub fn iter_hits<'a>(&'a self) -> impl Iterator<Item = SoAHitRef<Hit4>> + 'a {
        SoAHitIter::new(self, 4).filter(|h| h.hit())
    }
}

impl SoAHit for Hit4 {
    fn normal(&self, i: usize) -> Vector3<f32> {
        Vector3::new(self.Ng_x[i], self.Ng_y[i], self.Ng_z[i])
    }
    fn set_normal(&mut self, i: usize, n: Vector3<f32>) {
        self.Ng_x[i] = n.x;
        self.Ng_y[i] = n.y;
        self.Ng_z[i] = n.z;
    }

    fn uv(&self, i: usize) -> (f32, f32) {
        (self.u[i], self.v[i])
    }
    fn set_u(&mut self, i: usize, u: f32) {
        self.u[i] = u;
    }
    fn set_v(&mut self, i: usize, v: f32) {
        self.v[i] = v;
    }

    fn prim_id(&self, i: usize) -> u32 {
        self.primID[i]
    }
    fn set_prim_id(&mut self, i: usize, id: u32) {
        self.primID[i] = id;
    }

    fn geom_id(&self, i: usize) -> u32 {
        self.geomID[i]
    }
    fn set_geom_id(&mut self, i: usize, id: u32) {
        self.geomID[i] = id;
    }

    fn inst_id(&self, i: usize) -> u32 {
        self.instID[0][i]
    }
    fn set_inst_id(&mut self, i: usize, id: u32) {
        self.instID[0][i] = id;
    }
}

impl RayHit4 {
    pub fn new(ray: Ray4) -> RayHit4 {
        sys::RTCRayHit4 {
            ray: ray,
            hit: Hit4::new(),
        }
    }
    pub fn iter(&self) -> std::iter::Zip<SoARayIter<Ray4>, SoAHitIter<Hit4>> {
        self.ray.iter().zip(self.hit.iter())
    }
}
