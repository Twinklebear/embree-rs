use cgmath::Vector3;
use std::{f32, u32};
use std::iter::Iterator;
use std::marker::PhantomData;

pub use soa_ray::{SoARay, SoAHit, SoARayRef, SoARayRefMut,
                    SoARayIter, SoARayIterMut, SoAHitRef, SoAHitIter,
                    SoAHitIterMut};
use sys;

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
    pub fn with_capacity(n: usize) -> RayN {
        RayN {
            org_x: vec![0.0; n],
            org_y: vec![0.0; n],
            org_z: vec![0.0; n],
            dir_x: vec![0.0; n],
            dir_y: vec![0.0; n],
            dir_z: vec![0.0; n],
            tnear: vec![0.0; n],
            tfar: vec![f32::INFINITY; n],
            time: vec![0.0; n],
            mask: vec![u32::MAX; n],
            id: vec![0; n],
            flags: vec![0; n],
        }
    }
    pub fn new(origin: Vec<Vector3<f32>>, dir: Vec<Vector3<f32>>) -> RayN {
        let n = origin.len();
        RayN::segment(origin, dir, vec![0.0; n], vec![f32::INFINITY; n])
    }
    pub fn segment(origin: Vec<Vector3<f32>>, dir: Vec<Vector3<f32>>,
                   tnear: Vec<f32>, tfar: Vec<f32>) -> RayN {
        assert_eq!(origin.len(), dir.len());

        let n = origin.len();
        let mut org_x = Vec::with_capacity(n);
        let mut org_y = Vec::with_capacity(n);
        let mut org_z = Vec::with_capacity(n);
        for v in origin.iter() {
            org_x.push(v.x);
            org_y.push(v.y);
            org_z.push(v.z);
        }

        let mut dir_x = Vec::with_capacity(n);
        let mut dir_y = Vec::with_capacity(n);
        let mut dir_z = Vec::with_capacity(n);
        for v in dir.iter() {
            dir_x.push(v.x);
            dir_y.push(v.y);
            dir_z.push(v.z);
        }

        RayN {
            org_x: org_x,
            org_y: org_y,
            org_z: org_z,
            dir_x: dir_x,
            dir_y: dir_y,
            dir_z: dir_z,
            tnear: tnear,
            tfar: tfar,
            time: vec![0.0; n],
            mask: vec![u32::MAX; n],
            id: vec![0; n],
            flags: vec![0; n],
        }
    }
    pub fn iter(&self) -> SoARayIter<RayN> {
        SoARayIter::new(self, self.len())
    }
    pub fn iter_mut(&mut self) -> SoARayIterMut<RayN> {
        let n = self.len();
        SoARayIterMut::new(self, n)
    }
    pub fn len(&self) -> usize { self.org_x.len() }
    pub(crate) unsafe fn as_raynp(&mut self) -> sys::RTCRayNp {
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

    fn tnear(&self, i: usize) -> f32 { self.tnear[i] }
    fn set_tnear(&mut self, i: usize, near: f32) {
        self.tnear[i] = near;
    }

    fn tfar(&self, i: usize) -> f32 { self.tfar[i] }
    fn set_tfar(&mut self, i: usize, far: f32) {
        self.tfar[i] = far;
    }

    fn time(&self, i: usize) -> f32 { self.time[i] }
    fn set_time(&mut self, i: usize, time: f32) {
        self.time[i] = time;
    }

    fn mask(&self, i: usize) -> u32 { self.mask[i] }
    fn set_mask(&mut self, i: usize, mask: u32) {
        self.mask[i] = mask;
    }

    fn id(&self, i: usize) -> u32 { self.id[i] }
    fn set_id(&mut self, i: usize, id: u32) {
        self.id[i] = id;
    }

    fn flags(&self, i: usize) -> u32 { self.flags[i] }
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
            ng_x: vec![0.0; n],
            ng_y: vec![0.0; n],
            ng_z: vec![0.0; n],
            u: vec![0.0; n],
            v: vec![0.0; n],
            prim_id: vec![u32::MAX; n],
            geom_id: vec![u32::MAX; n],
            inst_id: vec![u32::MAX; n],
        }
    }
    pub fn any_hit(&self) -> bool {
        self.hits().fold(false, |acc, g| acc || g)
    }
    pub fn hits<'a>(&'a self) -> impl Iterator<Item=bool> + 'a {
        self.geom_id.iter().map(|g| *g != u32::MAX)
    }
    pub fn iter(&self) -> SoAHitIter<HitN> {
        SoAHitIter::new(self, self.len())
    }
    pub fn iter_hits<'a>(&'a self) -> impl Iterator<Item=SoAHitRef<HitN>> + 'a {
        SoAHitIter::new(self, self.len()).filter(|h| h.hit())
    }
    pub fn len(&self) -> usize { self.ng_x.len() }
    pub(crate) unsafe fn as_hitnp(&mut self) -> sys::RTCHitNp {
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
    fn normal(&self, i: usize) -> Vector3<f32> {
        Vector3::new(self.ng_x[i], self.ng_y[i], self.ng_z[i])
    }
    fn set_normal(&mut self, i: usize, n: Vector3<f32>) {
        self.ng_x[i] = n.x;
        self.ng_y[i] = n.y;
        self.ng_z[i] = n.z;
    }

    fn uv(&self, i: usize) -> (f32, f32) { (self.u[i], self.v[i]) }
    fn set_u(&mut self, i: usize, u: f32) {
        self.u[i] = u;
    }
    fn set_v(&mut self, i: usize, v: f32) {
        self.v[i] = v;
    }

    fn prim_id(&self, i: usize) -> u32 { self.prim_id[i] }
    fn set_prim_id(&mut self, i: usize, id: u32) {
        self.prim_id[i] = id;
    }

    fn geom_id(&self, i: usize) -> u32 { self.geom_id[i] }
    fn set_geom_id(&mut self, i: usize, id: u32) {
        self.geom_id[i] = id;
    }

    fn inst_id(&self, i: usize) -> u32 { self.inst_id[i] }
    fn set_inst_id(&mut self, i: usize, id: u32) {
        self.inst_id[i] = id;
    }
}

pub struct RayHitN {
    pub rays: RayN,
    pub hits: HitN,
}

impl RayHitN {
    pub fn new(rays: RayN) -> RayHitN {
        let n = rays.len();
        RayHitN {
            rays: rays,
            hits: HitN::new(n),
        }
    }
    pub fn iter(&self) -> std::iter::Zip<SoARayIter<RayN>, SoAHitIter<HitN>> {
        self.rays.iter().zip(self.hits.iter())
    }
    pub fn len(&self) -> usize { self.rays.len() }
}

