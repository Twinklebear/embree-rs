use std::iter::Iterator;
use std::{f32, u32};

use crate::soa_ray::{SoAHit, SoAHitIter, SoAHitRef, SoARay, SoARayIter, SoARayIterMut};
use crate::sys;
use crate::{aligned_vector, aligned_vector_init};

/// A ray stream stored in SoA format
pub struct RayN {
    org_x: Vec<f32>,
    org_y: Vec<f32>,
    org_z: Vec<f32>,
    tnear: Vec<f32>,
    dir_x: Vec<f32>,
    dir_y: Vec<f32>,
    dir_z: Vec<f32>,
    time: Vec<f32>,
    tfar: Vec<f32>,
    mask: Vec<::std::os::raw::c_uint>,
    id: Vec<::std::os::raw::c_uint>,
    flags: Vec<::std::os::raw::c_uint>,
}

impl RayN {
    /// Allocate a new Ray stream with room for `n` rays
    pub fn new(n: usize) -> RayN {
        RayN {
            org_x: aligned_vector::<f32>(n, 16),
            org_y: aligned_vector::<f32>(n, 16),
            org_z: aligned_vector::<f32>(n, 16),
            tnear: aligned_vector_init::<f32>(n, 16, 0.0),
            dir_x: aligned_vector::<f32>(n, 16),
            dir_y: aligned_vector::<f32>(n, 16),
            dir_z: aligned_vector::<f32>(n, 16),
            time: aligned_vector_init::<f32>(n, 16, 0.0),
            tfar: aligned_vector_init::<f32>(n, 16, f32::INFINITY),
            mask: aligned_vector_init::<u32>(n, 16, u32::MAX),
            id: aligned_vector_init::<u32>(n, 16, 0),
            flags: aligned_vector_init::<u32>(n, 16, 0),
        }
    }
    pub fn iter(&self) -> SoARayIter<RayN> {
        SoARayIter::new(self, self.len())
    }
    pub fn iter_mut(&mut self) -> SoARayIterMut<RayN> {
        let n = self.len();
        SoARayIterMut::new(self, n)
    }
    pub fn len(&self) -> usize {
        self.org_x.len()
    }
    pub unsafe fn as_raynp(&mut self) -> sys::RTCRayNp {
        sys::RTCRayNp {
            org_x: self.org_x.as_mut_ptr(),
            org_y: self.org_y.as_mut_ptr(),
            org_z: self.org_z.as_mut_ptr(),
            dir_x: self.dir_x.as_mut_ptr(),
            dir_y: self.dir_y.as_mut_ptr(),
            dir_z: self.dir_z.as_mut_ptr(),
            tnear: self.tnear.as_mut_ptr(),
            tfar: self.tfar.as_mut_ptr(),
            time: self.time.as_mut_ptr(),
            mask: self.mask.as_mut_ptr(),
            id: self.id.as_mut_ptr(),
            flags: self.flags.as_mut_ptr(),
        }
    }
}

impl SoARay for RayN {
    fn org(&self, i: usize) -> [f32; 3] {
        [self.org_x[i], self.org_y[i], self.org_z[i]]
    }
    fn set_org(&mut self, i: usize, o: [f32; 3]) {
        self.org_x[i] = o[0];
        self.org_y[i] = o[1];
        self.org_z[i] = o[2];
    }

    fn dir(&self, i: usize) -> [f32; 3] {
        [self.dir_x[i], self.dir_y[i], self.dir_z[i]]
    }
    fn set_dir(&mut self, i: usize, d: [f32; 3]) {
        self.dir_x[i] = d[0];
        self.dir_y[i] = d[1];
        self.dir_z[i] = d[2];
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

pub struct HitN {
    ng_x: Vec<f32>,
    ng_y: Vec<f32>,
    ng_z: Vec<f32>,
    u: Vec<f32>,
    v: Vec<f32>,
    prim_id: Vec<::std::os::raw::c_uint>,
    geom_id: Vec<::std::os::raw::c_uint>,
    inst_id: Vec<::std::os::raw::c_uint>,
}

impl HitN {
    pub fn new(n: usize) -> HitN {
        HitN {
            ng_x: aligned_vector::<f32>(n, 16),
            ng_y: aligned_vector::<f32>(n, 16),
            ng_z: aligned_vector::<f32>(n, 16),
            u: aligned_vector::<f32>(n, 16),
            v: aligned_vector::<f32>(n, 16),
            prim_id: aligned_vector_init::<u32>(n, 16, u32::MAX),
            geom_id: aligned_vector_init::<u32>(n, 16, u32::MAX),
            inst_id: aligned_vector_init::<u32>(n, 16, u32::MAX),
        }
    }
    pub fn any_hit(&self) -> bool {
        self.hits().fold(false, |acc, g| acc || g)
    }
    pub fn hits<'a>(&'a self) -> impl Iterator<Item = bool> + 'a {
        self.geom_id.iter().map(|g| *g != u32::MAX)
    }
    pub fn iter(&self) -> SoAHitIter<HitN> {
        SoAHitIter::new(self, self.len())
    }
    pub fn iter_hits<'a>(&'a self) -> impl Iterator<Item = SoAHitRef<HitN>> + 'a {
        SoAHitIter::new(self, self.len()).filter(|h| h.hit())
    }
    pub fn len(&self) -> usize {
        self.ng_x.len()
    }
    pub unsafe fn as_hitnp(&mut self) -> sys::RTCHitNp {
        sys::RTCHitNp {
            Ng_x: self.ng_x.as_mut_ptr(),
            Ng_y: self.ng_y.as_mut_ptr(),
            Ng_z: self.ng_z.as_mut_ptr(),
            u: self.u.as_mut_ptr(),
            v: self.v.as_mut_ptr(),
            primID: self.prim_id.as_mut_ptr(),
            geomID: self.geom_id.as_mut_ptr(),
            instID: [self.inst_id.as_mut_ptr(); 1usize],
        }
    }
}

impl SoAHit for HitN {
    fn normal(&self, i: usize) -> [f32; 3] {
        [self.ng_x[i], self.ng_y[i], self.ng_z[i]]
    }
    fn set_normal(&mut self, i: usize, n: [f32; 3]) {
        self.ng_x[i] = n[0];
        self.ng_y[i] = n[1];
        self.ng_z[i] = n[2];
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
        self.prim_id[i]
    }
    fn set_prim_id(&mut self, i: usize, id: u32) {
        self.prim_id[i] = id;
    }

    fn geom_id(&self, i: usize) -> u32 {
        self.geom_id[i]
    }
    fn set_geom_id(&mut self, i: usize, id: u32) {
        self.geom_id[i] = id;
    }

    fn inst_id(&self, i: usize) -> u32 {
        self.inst_id[i]
    }
    fn set_inst_id(&mut self, i: usize, id: u32) {
        self.inst_id[i] = id;
    }
}

pub struct RayHitN {
    pub ray: RayN,
    pub hit: HitN,
}

impl RayHitN {
    pub fn new(ray: RayN) -> RayHitN {
        let n = ray.len();
        RayHitN {
            ray: ray,
            hit: HitN::new(n),
        }
    }
    pub fn iter(&self) -> std::iter::Zip<SoARayIter<RayN>, SoAHitIter<HitN>> {
        self.ray.iter().zip(self.hit.iter())
    }
    pub fn len(&self) -> usize {
        self.ray.len()
    }
    pub unsafe fn as_rayhitnp(&mut self) -> sys::RTCRayHitNp {
        sys::RTCRayHitNp {
            ray: self.ray.as_raynp(),
            hit: self.hit.as_hitnp(),
        }
    }
}
