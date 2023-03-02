use crate::{
    sys, Hit, Ray, RayHit, SoAHit, SoAHitIter, SoAHitRef, SoARay, SoARayIter, SoARayIterMut,
    INVALID_ID,
};
use std::marker::PhantomData;

mod sealed {
    pub trait Sealed {}
}

/// A ray packet of size 4.
pub type Ray4 = sys::RTCRay4;

/// A hit packet of size 4.
pub type Hit4 = sys::RTCHit4;

/// A ray/hit packet of size 4.
pub type RayHit4 = sys::RTCRayHit4;

/// A ray packet of size 8.
pub type Ray8 = sys::RTCRay8;

/// A hit packet of size 8.
pub type Hit8 = sys::RTCHit8;

/// A ray/hit packet of size 8.
pub type RayHit8 = sys::RTCRayHit8;

/// A ray packet of size 16.
pub type Ray16 = sys::RTCRay16;

/// A hit packet of size 16.
pub type Hit16 = sys::RTCHit16;

/// A ray/hit packet of size 16.
pub type RayHit16 = sys::RTCRayHit16;

/// Represents a packet of rays.
///
/// Used as a trait bound for functions that operate on ray packets.
/// See [`Scene::occluded_stream_aos`] and [`Scene::intersect_stream_aos`].
pub trait RayPacket: Sized + sealed::Sealed {
    const LEN: usize;
}

pub trait HitPacket: Sized + sealed::Sealed {
    const LEN: usize;
}

pub trait RayHitPacket: Sized + sealed::Sealed {
    type Ray: RayPacket;
    type Hit: HitPacket;
    const LEN: usize = Self::Ray::LEN;
}

macro_rules! impl_packet_traits {
    ($($ray:ident, $hit:ident, $rayhit:ident, $n:expr);*) => {
        $(
            impl sealed::Sealed for $ray {}
            impl RayPacket for $ray {
                const LEN: usize = $n;
            }

            impl sealed::Sealed for $hit {}
            impl HitPacket for $hit {
                const LEN: usize = $n;
            }

            impl sealed::Sealed for $rayhit {}
            impl RayHitPacket for $rayhit {
                type Ray = $ray;
                type Hit = $hit;
            }
        )*
    }
}

impl_packet_traits! {
    Ray, Hit, RayHit, 1;
    Ray4, Hit4, RayHit4, 4;
    Ray8, Hit8, RayHit8, 8;
    Ray16, Hit16, RayHit16, 16
}

macro_rules! impl_ray_packets {
    ($($t:ident, $n:expr);*) => {
        $(
            impl $t {
                pub const fn new(origin: [[f32; 3]; $n], dir: [[f32; 3]; $n]) -> $t {
                    $t::segment(origin, dir, [0.0; $n], [f32::INFINITY; $n])
                }

                pub const fn segment(origin: [[f32; 3]; $n], dir: [[f32; 3]; $n], tnear: [f32; $n], tfar: [f32; $n]) -> $t {
                    let [org_x, org_y, org_z, dir_x, dir_y, dir_z] = {
                        let mut elems = [[0.0f32; $n]; 6];
                        let mut i = 0;
                        while i < $n {
                            elems[0][i] = origin[i][0];
                            elems[1][i] = origin[i][1];
                            elems[2][i] = origin[i][2];
                            elems[3][i] = dir[i][0];
                            elems[4][i] = dir[i][1];
                            elems[5][i] = dir[i][2];
                            i += 1;
                        }
                        elems
                    };
                    Self {
                        org_x,
                        org_y,
                        org_z,
                        dir_x,
                        dir_y,
                        dir_z,
                        tnear,
                        tfar,
                        time: [0.0; $n],
                        mask: [u32::MAX; $n],
                        id: [0; $n],
                        flags: [0; $n],
                    }
                }

                pub const fn empty() -> $t {
                    $t::segment(
                        [[0.0, 0.0, 0.0]; $n],
                        [[0.0, 0.0, 0.0]; $n],
                        [0.0; $n],
                        [f32::INFINITY; $n],
                    )
                }

                pub fn iter(&self) -> SoARayIter<$t> { SoARayIter::new(self, $n) }

                pub fn iter_mut(&mut self) -> SoARayIterMut<$t> { SoARayIterMut::new(self, $n) }
            }

            impl Default for $t {
                fn default() -> Self { Self::empty() }
            }

            impl SoARay for $t {
                fn org(&self, i: usize) -> [f32; 3] { [self.org_x[i], self.org_y[i], self.org_z[i]] }
                fn set_org(&mut self, i: usize, o: [f32; 3]) {
                    self.org_x[i] = o[0];
                    self.org_y[i] = o[1];
                    self.org_z[i] = o[2];
                }

                fn dir(&self, i: usize) -> [f32; 3] { [self.dir_x[i], self.dir_y[i], self.dir_z[i]] }
                fn set_dir(&mut self, i: usize, d: [f32; 3]) {
                    self.dir_x[i] = d[0];
                    self.dir_y[i] = d[1];
                    self.dir_z[i] = d[2];
                }

                fn tnear(&self, i: usize) -> f32 { self.tnear[i] }
                fn set_tnear(&mut self, i: usize, t: f32) { self.tnear[i] = t }

                fn tfar(&self, i: usize) -> f32 { self.tfar[i] }
                fn set_tfar(&mut self, i: usize, t: f32) { self.tfar[i] = t}

                fn time(&self, i: usize) -> f32 { self.time[i] }
                fn set_time(&mut self, i: usize, t: f32) { self.time[i] = t }

                fn mask(&self, i: usize) -> u32 { self.mask[i] }
                fn set_mask(&mut self, i: usize, m: u32) { self.mask[i] = m }

                fn id(&self, i: usize) -> u32 { self.id[i] }
                fn set_id(&mut self, i: usize, id: u32) { self.id[i] = id }

                fn flags(&self, i: usize) -> u32 { self.flags[i] }
                fn set_flags(&mut self, i: usize, f: u32) { self.flags[i] = f }
            }
        )*
    };
}

impl_ray_packets!(Ray4, 4; Ray8, 8; Ray16, 16);

macro_rules! impl_hit_packets {
    ($($t:ident, $n:expr);*) => {
        $(
            impl $t {
                pub fn new() -> $t {
                    $t {
                        Ng_x: [0.0; $n],
                        Ng_y: [0.0; $n],
                        Ng_z: [0.0; $n],
                        u: [0.0; $n],
                        v: [0.0; $n],
                        primID: [INVALID_ID; $n],
                        geomID: [INVALID_ID; $n],
                        instID: [[INVALID_ID; $n]],
                    }
                }
                pub fn any_hit(&self) -> bool { self.hits().any(|h| h) }
                pub fn hits(&self) -> impl Iterator<Item = bool> + '_ {
                    self.geomID.iter().map(|g| *g != INVALID_ID)
                }
                pub fn iter(&self) -> SoAHitIter<$t> { SoAHitIter::new(self, $n) }
                pub fn iter_hits(&self) -> impl Iterator<Item = SoAHitRef<$t>> {
                    SoAHitIter::new(self, 4).filter(|h| h.hit())
                }
            }

            impl Default for $t {
                fn default() -> Self { Self::new() }
            }

            impl SoAHit for $t {
                fn normal(&self, i: usize) -> [f32; 3] { [self.Ng_x[i], self.Ng_y[i], self.Ng_z[i]] }
                fn set_normal(&mut self, i: usize, n: [f32; 3]) {
                    self.Ng_x[i] = n[0];
                    self.Ng_y[i] = n[1];
                    self.Ng_z[i] = n[2];
                }

                fn uv(&self, i: usize) -> (f32, f32) { (self.u[i], self.v[i]) }
                fn set_u(&mut self, i: usize, u: f32) { self.u[i] = u; }
                fn set_v(&mut self, i: usize, v: f32) { self.v[i] = v; }

                fn prim_id(&self, i: usize) -> u32 { self.primID[i] }
                fn set_prim_id(&mut self, i: usize, id: u32) { self.primID[i] = id; }

                fn geom_id(&self, i: usize) -> u32 { self.geomID[i] }
                fn set_geom_id(&mut self, i: usize, id: u32) { self.geomID[i] = id; }

                fn inst_id(&self, i: usize) -> u32 { self.instID[0][i] }
                fn set_inst_id(&mut self, i: usize, id: u32) { self.instID[0][i] = id; }
            }
        )*
    };
}

impl_hit_packets!(Hit4, 4; Hit8, 8; Hit16, 16);

impl RayHit4 {
    pub fn new(ray: Ray4) -> RayHit4 {
        sys::RTCRayHit4 {
            ray,
            hit: Hit4::new(),
        }
    }
    pub fn iter(&self) -> std::iter::Zip<SoARayIter<Ray4>, SoAHitIter<Hit4>> {
        self.ray.iter().zip(self.hit.iter())
    }
}

/// Ray packet of runtime size.
///
/// It is used to represent a packet of rays that is not known at compile
/// time, generally used as an argument to callback functions. The size
/// of the packet can only be either 1, 4, 8, or 16.
///
/// For ray streams, use [`RayNp`](`crate::ray::RayNp`).
pub struct RayN<'a> {
    pub(crate) ptr: *mut sys::RTCRayN,
    pub(crate) len: usize,
    pub(crate) marker: PhantomData<&'a mut sys::RTCRayN>,
}

impl<'a> RayN<'a> {
    pub const fn org(&self, i: usize) -> [f32; 3] {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe {
            let ptr = self.ptr as *const f32;
            [
                *ptr.add(i),
                *ptr.add(self.len + i),
                *ptr.add(2 * self.len + i),
            ]
        }
    }

    pub const fn org_x(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(i) }
    }

    pub fn set_org_x(&mut self, i: usize, x: f32) {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe {
            *(self.ptr as *mut f32).add(i) = x;
        }
    }

    pub const fn org_y(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(self.len + i) }
    }

    pub fn set_org_y(&mut self, i: usize, y: f32) {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe {
            *(self.ptr as *mut f32).add(self.len + i) = y;
        }
    }

    pub const fn org_z(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(2 * self.len + i) }
    }

    pub fn set_org_z(&mut self, i: usize, z: f32) {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe {
            *(self.ptr as *mut f32).add(2 * self.len + i) = z;
        }
    }

    pub const fn tnear(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(3 * self.len + i) }
    }

    pub fn set_tnear(&mut self, i: usize, t: f32) {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe {
            *(self.ptr as *mut f32).add(3 * self.len + i) = t;
        }
    }

    pub const fn dir(&self, i: usize) -> [f32; 3] {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe {
            let ptr = self.ptr as *const f32;
            [
                *ptr.add(4 * self.len + i),
                *ptr.add(5 * self.len + i),
                *ptr.add(6 * self.len + i),
            ]
        }
    }

    pub const fn dir_x(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(4 * self.len + i) }
    }

    pub fn set_dir_x(&mut self, i: usize, x: f32) {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe {
            *(self.ptr as *mut f32).add(4 * self.len + i) = x;
        }
    }

    pub const fn dir_y(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(5 * self.len + i) }
    }

    pub fn set_dir_y(&mut self, i: usize, y: f32) {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe {
            *(self.ptr as *mut f32).add(5 * self.len + i) = y;
        }
    }

    pub const fn dir_z(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(6 * self.len + i) }
    }

    pub fn set_dir_z(&mut self, i: usize, z: f32) {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe {
            *(self.ptr as *mut f32).add(6 * self.len + i) = z;
        }
    }

    pub const fn time(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(7 * self.len + i) }
    }

    pub fn set_time(&mut self, i: usize, t: f32) {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe {
            *(self.ptr as *mut f32).add(7 * self.len + i) = t;
        }
    }

    pub const fn tfar(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(8 * self.len + i) }
    }

    pub fn set_tfar(&mut self, i: usize, t: f32) {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe {
            *(self.ptr as *mut f32).add(8 * self.len + i) = t;
        }
    }

    pub const fn mask(&self, i: usize) -> u32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const u32).add(9 * self.len + i) }
    }

    pub fn set_mask(&mut self, i: usize, m: u32) {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe {
            *(self.ptr as *mut u32).add(9 * self.len + i) = m;
        }
    }

    pub const fn id(&self, i: usize) -> u32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const u32).add(10 * self.len + i) }
    }

    pub fn set_id(&mut self, i: usize, id: u32) {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe {
            *(self.ptr as *mut u32).add(10 * self.len + i) = id;
        }
    }

    pub const fn flags(&self, i: usize) -> u32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const u32).add(11 * self.len + i) }
    }

    pub fn set_flags(&mut self, i: usize, f: u32) {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe {
            *(self.ptr as *mut u32).add(11 * self.len + i) = f;
        }
    }

    pub const fn len(&self) -> usize { self.len }
}

/// Hit packet of runtime size.
///
/// It is used to represent a packet of hits that is not known at compile
/// time, generally used as an argument to callback functions. The size
/// of the packet can only be either 1, 4, 8, or 16.
pub struct HitN<'a> {
    pub(crate) ptr: *mut sys::RTCHitN,
    pub(crate) len: usize,
    pub(crate) marker: PhantomData<&'a mut sys::RTCHitN>,
}

impl<'a> HitN<'a> {
    pub const fn ng_x(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(i) }
    }

    pub const fn ng_y(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(self.len + i) }
    }

    pub const fn ng_z(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(2 * self.len + i) }
    }

    pub const fn u(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(3 * self.len + i) }
    }

    pub const fn v(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(4 * self.len + i) }
    }

    pub const fn prim_id(&self, i: usize) -> u32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const u32).add(5 * self.len + i) }
    }

    pub const fn geom_id(&self, i: usize) -> u32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const u32).add(6 * self.len + i) }
    }

    pub const fn inst_id(&self, i: usize) -> u32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const u32).add(7 * self.len + i) }
    }

    pub const fn len(&self) -> usize { self.len }
}

/// Combined ray and hit packet of runtime size.
///
/// The size of the packet can only be either 1, 4, 8, or 16.
pub struct RayHitN<'a> {
    pub(crate) ptr: *mut sys::RTCRayHitN,
    pub(crate) len: usize,
    pub(crate) marker: PhantomData<&'a mut sys::RTCRayHitN>,
}

impl<'a> RayHitN<'a> {
    /// Returns the ray packet.
    pub fn ray_n(&'a self) -> RayN<'a> {
        RayN {
            ptr: self.ptr as *mut sys::RTCRayN,
            len: self.len,
            marker: PhantomData,
        }
    }

    /// Returns the hit packet.
    pub fn hit_n(&'a self) -> HitN<'a> {
        HitN {
            ptr: unsafe { (self.ptr as *const u32).add(12 * self.len) as *mut sys::RTCHitN },
            len: self.len,
            marker: PhantomData,
        }
    }
}
