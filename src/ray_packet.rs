use cgmath::Vector3;
use std::{f32, u32};
use std::iter::Iterator;
use std::marker::PhantomData;

use sys;

pub type Ray4 = sys::RTCRay4;
pub type Hit4 = sys::RTCHit4;
pub type RayHit4 = sys::RTCRayHit4;

impl Ray4 {
    pub fn empty() -> Ray4 {
        Ray4::segment([Vector3::new(0.0, 0.0, 0.0); 4],
                      [Vector3::new(0.0, 0.0, 0.0); 4],
                      [0.0; 4], [f32::INFINITY; 4])
    }
    pub fn new(origin: [Vector3<f32>; 4], dir: [Vector3<f32>; 4]) -> Ray4 {
        Ray4::segment(origin, dir, [0.0; 4], [f32::INFINITY; 4])
    }
    pub fn segment(origin: [Vector3<f32>; 4], dir: [Vector3<f32>; 4],
                   tnear: [f32; 4], tfar: [f32; 4]) -> Ray4 {
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
    pub fn iter(&self) -> Ray4Iter {
        Ray4Iter::new(self)
    }
    pub fn iter_mut(&mut self) -> Ray4IterMut {
        Ray4IterMut::new(self)
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
    pub fn iter(&self) -> Hit4Iter {
        Hit4Iter::new(self)
    }
    pub fn iter_hits<'a>(&'a self) -> impl Iterator<Item=HitRef> + 'a {
        Hit4Iter::new(self).filter(|h| h.hit())
    }
}

impl RayHit4 {
    pub fn new(ray: Ray4) -> RayHit4 {
        sys::RTCRayHit4 {
            ray: ray,
            hit: Hit4::new(),
        }
    }
    pub fn iter(&self) -> std::iter::Zip<Ray4Iter, Hit4Iter> {
        self.ray.iter().zip(self.hit.iter())
    }
}

pub struct RayRef<'a> {
    packet: &'a Ray4,
    ray: usize,
}

impl<'a> RayRef<'a> {
    pub fn origin(&self) -> Vector3<f32> {
        Vector3::new(self.packet.org_x[self.ray],
                     self.packet.org_y[self.ray],
                     self.packet.org_z[self.ray])
    }
    pub fn dir(&self) -> Vector3<f32> {
        Vector3::new(self.packet.dir_x[self.ray],
                     self.packet.dir_y[self.ray],
                     self.packet.dir_z[self.ray])
    }
    pub fn tnear(&self) -> f32 {
        self.packet.tnear[self.ray]
    }
    pub fn tfar(&self) -> f32 {
        self.packet.tfar[self.ray]
    }
    pub fn mask(&self) -> u32 {
        self.packet.mask[self.ray] as u32
    }
    pub fn id(&self) -> u32 {
        self.packet.id[self.ray] as u32
    }
    pub fn flags(&self) -> u32 {
        self.packet.flags[self.ray] as u32
    }
}

pub struct Ray4Iter<'a> {
    packet: &'a Ray4,
    cur: usize,
}

impl<'a> Ray4Iter<'a> {
    fn new(packet: &'a Ray4) -> Ray4Iter<'a> {
        Ray4Iter { packet: packet, cur: 0 }
    }
}

impl<'a> Iterator for Ray4Iter<'a> {
    type Item = RayRef<'a>;

    fn next(&mut self) -> Option<RayRef<'a>> {
        if self.cur == 4 {
            None
        } else {
            let i = self.cur;
            self.cur = self.cur + 1;
            Some(RayRef { packet: self.packet, ray: i})
        }
    }
}

// TODO: Is this going to work well?
pub struct RayRefMut<'a> {
    packet: *mut Ray4,
    ray: usize,
    marker: PhantomData<&'a mut Ray4>
}

impl<'a> RayRefMut<'a> {
    pub fn origin(&self) -> Vector3<f32> {
        let packet = unsafe { self.packet.as_ref().expect("should never be null!") };
        Vector3::new(packet.org_x[self.ray],
                     packet.org_y[self.ray],
                     packet.org_z[self.ray])
    }
    pub fn set_origin(&mut self, o: Vector3<f32>) {
        let packet = unsafe { self.packet.as_mut().expect("should never be null!") };
        packet.org_x[self.ray] = o.x;
        packet.org_y[self.ray] = o.y;
        packet.org_z[self.ray] = o.z;
    }
    pub fn dir(&self) -> Vector3<f32> {
        let packet = unsafe { self.packet.as_ref().expect("should never be null!") };
        Vector3::new(packet.dir_x[self.ray],
                     packet.dir_y[self.ray],
                     packet.dir_z[self.ray])
    }
    pub fn set_dir(&mut self, d: Vector3<f32>) {
        let packet = unsafe { self.packet.as_mut().expect("should never be null!") };
        packet.dir_x[self.ray] = d.x;
        packet.dir_y[self.ray] = d.y;
        packet.dir_z[self.ray] = d.z;
    }
    pub fn tnear(&self) -> f32 {
        let packet = unsafe { self.packet.as_ref().expect("should never be null!") };
        packet.tnear[self.ray]
    }
    pub fn set_tnear(&mut self, tnear: f32) {
        let packet = unsafe { self.packet.as_mut().expect("should never be null!") };
        packet.tnear[self.ray] = tnear;
    }
    pub fn tfar(&self) -> f32 {
        let packet = unsafe { self.packet.as_ref().expect("should never be null!") };
        packet.tfar[self.ray]
    }
    pub fn set_tfar(&mut self, tfar: f32) {
        let packet = unsafe { self.packet.as_mut().expect("should never be null!") };
        packet.tfar[self.ray] = tfar;
    }
    pub fn mask(&self) -> u32 {
        let packet = unsafe { self.packet.as_ref().expect("should never be null!") };
        packet.mask[self.ray] as u32
    }
    pub fn set_mask(&mut self, mask: u32) {
        let packet = unsafe { self.packet.as_mut().expect("should never be null!") };
        packet.mask[self.ray] = mask;
    }
    pub fn id(&self) -> u32 {
        let packet = unsafe { self.packet.as_ref().expect("should never be null!") };
        packet.id[self.ray] as u32
    }
    pub fn set_id(&mut self, id: u32) {
        let packet = unsafe { self.packet.as_mut().expect("should never be null!") };
        packet.id[self.ray] = id;
    }
    pub fn flags(&self) -> u32 {
        let packet = unsafe { self.packet.as_ref().expect("should never be null!") };
        packet.flags[self.ray] as u32
    }
    pub fn set_flags(&mut self, flags: u32) {
        let packet = unsafe { self.packet.as_mut().expect("should never be null!") };
        packet.flags[self.ray] = flags;
    }
}

pub struct Ray4IterMut<'a> {
    packet: &'a mut Ray4,
    cur: usize,
}

impl<'a> Ray4IterMut<'a> {
    fn new(packet: &'a mut Ray4) -> Ray4IterMut<'a> {
        Ray4IterMut { packet: packet, cur: 0 }
    }
}

impl<'a> Iterator for Ray4IterMut<'a> {
    type Item = RayRefMut<'a>;

    fn next(&mut self) -> Option<RayRefMut<'a>> {
        if self.cur == 4 {
            None
        } else {
            let i = self.cur;
            self.cur = self.cur + 1;
            Some(RayRefMut {
                packet: self.packet as *mut Ray4,
                ray: i,
                marker: PhantomData
            })
        }
    }
}

pub struct HitRef<'a> {
    hit: &'a Hit4,
    idx: usize,
}

impl<'a> HitRef<'a> {
    pub fn normal(&self) -> Vector3<f32> {
        Vector3::new(self.hit.Ng_x[self.idx],
                     self.hit.Ng_y[self.idx],
                     self.hit.Ng_z[self.idx])
    }
    pub fn uv(&self) -> (f32, f32) {
        (self.hit.u[self.idx], self.hit.v[self.idx])
    }
    pub fn prim_id(&self) -> u32 {
        self.hit.primID[self.idx]
    }
    pub fn geom_id(&self) -> u32 {
        self.hit.geomID[self.idx]
    }
    pub fn inst_id(&self) -> u32 {
        self.hit.instID[0][self.idx]
    }
    pub fn hit(&self) -> bool {
        self.hit.geomID[self.idx] != u32::MAX
    }
}

pub struct Hit4Iter<'a> {
    hit: &'a Hit4,
    cur: usize,
}

impl<'a> Hit4Iter<'a> {
    fn new(hit: &'a Hit4) -> Hit4Iter<'a> {
        Hit4Iter{ hit: hit, cur: 0 }
    }
}

impl<'a> Iterator for Hit4Iter<'a> {
    type Item = HitRef<'a>;

    fn next(&mut self) -> Option<HitRef<'a>> {
        if self.cur == 4 {
            None
        } else {
            let i = self.cur;
            self.cur = self.cur + 1;
            Some(HitRef { hit: self.hit, idx: i})
        }
    }
}

