use std::{
    iter::{ExactSizeIterator, Iterator},
    marker::PhantomData,
    u32,
};

pub trait SoARay {
    fn org(&self, i: usize) -> [f32; 3];
    fn set_org(&mut self, i: usize, o: [f32; 3]);

    fn dir(&self, i: usize) -> [f32; 3];
    fn set_dir(&mut self, i: usize, d: [f32; 3]);

    fn tnear(&self, i: usize) -> f32;
    fn set_tnear(&mut self, i: usize, near: f32);

    fn tfar(&self, i: usize) -> f32;
    fn set_tfar(&mut self, i: usize, far: f32);

    fn time(&self, i: usize) -> f32;
    fn set_time(&mut self, i: usize, time: f32);

    fn mask(&self, i: usize) -> u32;
    fn set_mask(&mut self, i: usize, mask: u32);

    fn id(&self, i: usize) -> u32;
    fn set_id(&mut self, i: usize, id: u32);

    fn flags(&self, i: usize) -> u32;
    fn set_flags(&mut self, i: usize, flags: u32);
}

pub trait SoAHit {
    fn normal(&self, i: usize) -> [f32; 3];
    fn set_normal(&mut self, i: usize, n: [f32; 3]);

    fn uv(&self, i: usize) -> (f32, f32);
    fn set_u(&mut self, i: usize, u: f32);
    fn set_v(&mut self, i: usize, v: f32);

    fn prim_id(&self, i: usize) -> u32;
    fn set_prim_id(&mut self, i: usize, id: u32);

    fn geom_id(&self, i: usize) -> u32;
    fn set_geom_id(&mut self, i: usize, id: u32);

    fn inst_id(&self, i: usize) -> u32;
    fn set_inst_id(&mut self, i: usize, id: u32);

    fn hit(&self, i: usize) -> bool { self.geom_id(i) != u32::MAX }
}

pub struct SoARayRef<'a, T> {
    ray: &'a T,
    idx: usize,
}

impl<'a, T: SoARay + 'a> SoARayRef<'a, T> {
    pub fn origin(&self) -> [f32; 3] { self.ray.org(self.idx) }
    pub fn dir(&self) -> [f32; 3] { self.ray.dir(self.idx) }
    pub fn tnear(&self) -> f32 { self.ray.tnear(self.idx) }
    pub fn tfar(&self) -> f32 { self.ray.tfar(self.idx) }
    pub fn mask(&self) -> u32 { self.ray.mask(self.idx) }
    pub fn id(&self) -> u32 { self.ray.id(self.idx) }
    pub fn flags(&self) -> u32 { self.ray.flags(self.idx) }
}

// TODO: Is this going to work well?
pub struct SoARayRefMut<'a, T> {
    ray: *mut T,
    idx: usize,
    marker: PhantomData<&'a mut T>,
}

impl<'a, T: SoARay + 'a> SoARayRefMut<'a, T> {
    pub fn origin(&self) -> [f32; 3] {
        let ray = unsafe { self.ray.as_ref().expect("should never be null!") };
        ray.org(self.idx)
    }
    pub fn set_origin(&mut self, o: [f32; 3]) {
        let ray = unsafe { self.ray.as_mut().expect("should never be null!") };
        ray.set_org(self.idx, o);
    }
    pub fn dir(&self) -> [f32; 3] {
        let ray = unsafe { self.ray.as_ref().expect("should never be null!") };
        ray.dir(self.idx)
    }
    pub fn set_dir(&mut self, d: [f32; 3]) {
        let ray = unsafe { self.ray.as_mut().expect("should never be null!") };
        ray.set_dir(self.idx, d);
    }
    pub fn tnear(&self) -> f32 {
        let ray = unsafe { self.ray.as_ref().expect("should never be null!") };
        ray.tnear(self.idx)
    }
    pub fn set_tnear(&mut self, tnear: f32) {
        let ray = unsafe { self.ray.as_mut().expect("should never be null!") };
        ray.set_tnear(self.idx, tnear);
    }
    pub fn tfar(&self) -> f32 {
        let ray = unsafe { self.ray.as_ref().expect("should never be null!") };
        ray.tfar(self.idx)
    }
    pub fn set_tfar(&mut self, tfar: f32) {
        let ray = unsafe { self.ray.as_mut().expect("should never be null!") };
        ray.set_tfar(self.idx, tfar);
    }
    pub fn mask(&self) -> u32 {
        let ray = unsafe { self.ray.as_ref().expect("should never be null!") };
        ray.mask(self.idx)
    }
    pub fn set_mask(&mut self, mask: u32) {
        let ray = unsafe { self.ray.as_mut().expect("should never be null!") };
        ray.set_mask(self.idx, mask);
    }
    pub fn id(&self) -> u32 {
        let ray = unsafe { self.ray.as_ref().expect("should never be null!") };
        ray.id(self.idx)
    }
    pub fn set_id(&mut self, id: u32) {
        let ray = unsafe { self.ray.as_mut().expect("should never be null!") };
        ray.set_id(self.idx, id);
    }
    pub fn flags(&self) -> u32 {
        let ray = unsafe { self.ray.as_ref().expect("should never be null!") };
        ray.flags(self.idx)
    }
    pub fn set_flags(&mut self, flags: u32) {
        let ray = unsafe { self.ray.as_mut().expect("should never be null!") };
        ray.set_flags(self.idx, flags);
    }
}

pub struct SoARayIter<'a, T> {
    ray: &'a T,
    cur: usize,
    len: usize,
}

impl<'a, T: SoARay + 'a> SoARayIter<'a, T> {
    pub fn new(ray: &'a T, len: usize) -> SoARayIter<'a, T> { SoARayIter { ray, cur: 0, len } }
}

impl<'a, T: SoARay + 'a> Iterator for SoARayIter<'a, T> {
    type Item = SoARayRef<'a, T>;

    fn next(&mut self) -> Option<SoARayRef<'a, T>> {
        if self.cur >= self.len {
            None
        } else {
            let i = self.cur;
            self.cur += 1;
            Some(SoARayRef {
                ray: self.ray,
                idx: i,
            })
        }
    }
}

impl<'a, T: SoARay + 'a> ExactSizeIterator for SoARayIter<'a, T> {
    fn len(&self) -> usize { self.len - self.cur }
}

pub struct SoARayIterMut<'a, T> {
    ray: &'a mut T,
    cur: usize,
    len: usize,
}

impl<'a, T: SoARay + 'a> SoARayIterMut<'a, T> {
    pub fn new(ray: &'a mut T, len: usize) -> SoARayIterMut<'a, T> {
        SoARayIterMut { ray, cur: 0, len }
    }
}

impl<'a, T: SoARay + 'a> Iterator for SoARayIterMut<'a, T> {
    type Item = SoARayRefMut<'a, T>;

    fn next(&mut self) -> Option<SoARayRefMut<'a, T>> {
        if self.cur >= self.len {
            None
        } else {
            let i = self.cur;
            self.cur += 1;
            Some(SoARayRefMut {
                ray: self.ray as *mut T,
                idx: i,
                marker: PhantomData,
            })
        }
    }
}

impl<'a, T: SoARay + 'a> ExactSizeIterator for SoARayIterMut<'a, T> {
    fn len(&self) -> usize { self.len - self.cur }
}

pub struct SoAHitRef<'a, T> {
    hit: &'a T,
    idx: usize,
}

impl<'a, T: SoAHit + 'a> SoAHitRef<'a, T> {
    pub fn normal(&self) -> [f32; 3] { self.hit.normal(self.idx) }
    pub fn uv(&self) -> (f32, f32) { self.hit.uv(self.idx) }
    pub fn prim_id(&self) -> u32 { self.hit.prim_id(self.idx) }
    pub fn geom_id(&self) -> u32 { self.hit.geom_id(self.idx) }
    pub fn inst_id(&self) -> u32 { self.hit.inst_id(self.idx) }
    pub fn hit(&self) -> bool { self.hit.hit(self.idx) }
}

pub struct SoAHitIter<'a, T> {
    hit: &'a T,
    cur: usize,
    len: usize,
}

impl<'a, T: SoAHit + 'a> SoAHitIter<'a, T> {
    pub fn new(hit: &'a T, len: usize) -> SoAHitIter<'a, T> { SoAHitIter { hit, cur: 0, len } }
}

impl<'a, T: SoAHit + 'a> Iterator for SoAHitIter<'a, T> {
    type Item = SoAHitRef<'a, T>;

    fn next(&mut self) -> Option<SoAHitRef<'a, T>> {
        if self.cur >= self.len {
            None
        } else {
            let i = self.cur;
            self.cur += 1;
            Some(SoAHitRef {
                hit: self.hit,
                idx: i,
            })
        }
    }
}

impl<'a, T: SoAHit + 'a> ExactSizeIterator for SoAHitIter<'a, T> {
    fn len(&self) -> usize { self.len - self.cur }
}

pub struct SoAHitRefMut<'a, T> {
    hit: *mut T,
    idx: usize,
    marker: PhantomData<&'a mut T>,
}

impl<'a, T: SoAHit + 'a> SoAHitRefMut<'a, T> {
    pub fn normal(&self) -> [f32; 3] {
        let hit = unsafe { self.hit.as_ref().expect("should never be null!") };
        hit.normal(self.idx)
    }
    pub fn set_normal(&mut self, n: [f32; 3]) {
        let hit = unsafe { self.hit.as_mut().expect("should never be null!") };
        hit.set_normal(self.idx, n)
    }
    pub fn uv(&self) -> (f32, f32) {
        let hit = unsafe { self.hit.as_ref().expect("should never be null!") };
        hit.uv(self.idx)
    }
    pub fn set_u(&mut self, u: f32) {
        let hit = unsafe { self.hit.as_mut().expect("should never be null!") };
        hit.set_u(self.idx, u);
    }
    pub fn set_v(&mut self, v: f32) {
        let hit = unsafe { self.hit.as_mut().expect("should never be null!") };
        hit.set_v(self.idx, v);
    }
    pub fn prim_id(&self) -> u32 {
        let hit = unsafe { self.hit.as_ref().expect("should never be null!") };
        hit.prim_id(self.idx)
    }
    pub fn set_prim_id(&mut self, id: u32) {
        let hit = unsafe { self.hit.as_mut().expect("should never be null!") };
        hit.set_prim_id(self.idx, id);
    }
    pub fn geom_id(&self) -> u32 {
        let hit = unsafe { self.hit.as_ref().expect("should never be null!") };
        hit.geom_id(self.idx)
    }
    pub fn set_geom_id(&mut self, id: u32) {
        let hit = unsafe { self.hit.as_mut().expect("should never be null!") };
        hit.set_geom_id(self.idx, id);
    }
    pub fn inst_id(&self) -> u32 {
        let hit = unsafe { self.hit.as_ref().expect("should never be null!") };
        hit.inst_id(self.idx)
    }
    pub fn set_inst_id(&mut self, id: u32) {
        let hit = unsafe { self.hit.as_mut().expect("should never be null!") };
        hit.set_inst_id(self.idx, id);
    }
    pub fn hit(&self) -> bool {
        let hit = unsafe { self.hit.as_ref().expect("should never be null!") };
        hit.hit(self.idx)
    }
}

pub struct SoAHitIterMut<'a, T> {
    hit: &'a mut T,
    cur: usize,
    len: usize,
}

impl<'a, T: SoAHit + 'a> SoAHitIterMut<'a, T> {
    pub fn new(hit: &'a mut T, len: usize) -> SoAHitIterMut<'a, T> {
        SoAHitIterMut { hit, cur: 0, len }
    }
}

impl<'a, T: SoAHit + 'a> Iterator for SoAHitIterMut<'a, T> {
    type Item = SoAHitRefMut<'a, T>;

    fn next(&mut self) -> Option<SoAHitRefMut<'a, T>> {
        if self.cur >= self.len {
            None
        } else {
            let i = self.cur;
            self.cur += 1;
            Some(SoAHitRefMut {
                hit: self.hit as *mut T,
                idx: i,
                marker: PhantomData,
            })
        }
    }
}

impl<'a, T: SoAHit + 'a> ExactSizeIterator for SoAHitIterMut<'a, T> {
    fn len(&self) -> usize { self.len - self.cur }
}
