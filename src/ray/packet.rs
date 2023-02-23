use crate::{
    sys, Hit, Ray, RayHit, SoAHit, SoAHitIter, SoAHitRef, SoARay, SoARayIter, SoARayIterMut,
};

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
                pub fn new(origin: [[f32; 3]; $n], dir: [[f32; 3]; $n]) -> $t {
                    $t::segment(origin, dir, [0.0; $n], [f32::INFINITY; $n])
                }

                pub fn empty() -> $t {
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
        )*
    };
}

impl Ray4 {
    pub fn segment(
        origin: [[f32; 3]; 4],
        dir: [[f32; 3]; 4],
        tnear: [f32; 4],
        tfar: [f32; 4],
    ) -> Ray4 {
        sys::RTCRay4 {
            org_x: [origin[0][0], origin[1][0], origin[2][0], origin[3][0]],
            org_y: [origin[0][1], origin[1][1], origin[2][1], origin[3][1]],
            org_z: [origin[0][2], origin[1][2], origin[2][2], origin[3][2]],
            dir_x: [dir[0][0], dir[1][0], dir[2][0], dir[3][0]],
            dir_y: [dir[0][1], dir[1][1], dir[2][1], dir[3][1]],
            dir_z: [dir[0][2], dir[1][2], dir[2][2], dir[3][2]],
            tnear,
            tfar,
            time: [0.0; 4],
            mask: [u32::MAX; 4],
            id: [0; 4],
            flags: [0; 4],
        }
    }
}

impl Ray8 {
    pub fn segment(
        origin: [[f32; 3]; 8],
        dir: [[f32; 3]; 8],
        tnear: [f32; 8],
        tfar: [f32; 8],
    ) -> Ray8 {
        Ray8 {
            org_x: [
                origin[0][0],
                origin[1][0],
                origin[2][0],
                origin[3][0],
                origin[4][0],
                origin[5][0],
                origin[6][0],
                origin[7][0],
            ],
            org_y: [
                origin[0][1],
                origin[1][1],
                origin[2][1],
                origin[3][1],
                origin[4][1],
                origin[5][1],
                origin[6][1],
                origin[7][1],
            ],
            org_z: [
                origin[0][2],
                origin[1][2],
                origin[2][2],
                origin[3][2],
                origin[4][2],
                origin[5][2],
                origin[6][2],
                origin[7][2],
            ],
            dir_x: [
                dir[0][0], dir[1][0], dir[2][0], dir[3][0], dir[4][0], dir[5][0], dir[6][0],
                dir[7][0],
            ],
            dir_y: [
                dir[0][1], dir[1][1], dir[2][1], dir[3][1], dir[4][1], dir[5][1], dir[6][1],
                dir[7][1],
            ],
            dir_z: [
                dir[0][2], dir[1][2], dir[2][2], dir[3][2], dir[4][2], dir[5][2], dir[6][2],
                dir[7][2],
            ],
            tnear,
            tfar,
            time: [0.0; 8],
            mask: [u32::MAX; 8],
            id: [0; 8],
            flags: [0; 8],
        }
    }
}

impl Ray16 {
    pub fn segment(
        origin: [[f32; 3]; 16],
        dir: [[f32; 3]; 16],
        tnear: [f32; 16],
        tfar: [f32; 16],
    ) -> Ray16 {
        Ray16 {
            org_x: [
                origin[0][0],
                origin[1][0],
                origin[2][0],
                origin[3][0],
                origin[4][0],
                origin[5][0],
                origin[6][0],
                origin[7][0],
                origin[8][0],
                origin[9][0],
                origin[10][0],
                origin[11][0],
                origin[12][0],
                origin[13][0],
                origin[14][0],
                origin[15][0],
            ],
            org_y: [
                origin[0][1],
                origin[1][1],
                origin[2][1],
                origin[3][1],
                origin[4][1],
                origin[5][1],
                origin[6][1],
                origin[7][1],
                origin[8][1],
                origin[9][1],
                origin[10][1],
                origin[11][1],
                origin[12][1],
                origin[13][1],
                origin[14][1],
                origin[15][1],
            ],
            org_z: [
                origin[0][2],
                origin[1][2],
                origin[2][2],
                origin[3][2],
                origin[4][2],
                origin[5][2],
                origin[6][2],
                origin[7][2],
                origin[8][2],
                origin[9][2],
                origin[10][2],
                origin[11][2],
                origin[12][2],
                origin[13][2],
                origin[14][2],
                origin[15][2],
            ],
            dir_x: [
                dir[0][0], dir[1][0], dir[2][0], dir[3][0], dir[4][0], dir[5][0], dir[6][0],
                dir[7][0], dir[8][0], dir[9][0], dir[10][0], dir[11][0], dir[12][0], dir[13][0],
                dir[14][0], dir[15][0],
            ],
            dir_y: [
                dir[0][1], dir[1][1], dir[2][1], dir[3][1], dir[4][1], dir[5][1], dir[6][1],
                dir[7][1], dir[8][1], dir[9][1], dir[10][1], dir[11][1], dir[12][1], dir[13][1],
                dir[14][1], dir[15][1],
            ],
            dir_z: [
                dir[0][2], dir[1][2], dir[2][2], dir[3][2], dir[4][2], dir[5][2], dir[6][2],
                dir[7][2], dir[8][2], dir[9][2], dir[10][2], dir[11][2], dir[12][2], dir[13][2],
                dir[14][2], dir[15][2],
            ],
            tnear,
            tfar,
            time: [0.0; 16],
            mask: [u32::MAX; 16],
            id: [0; 16],
            flags: [0; 16],
        }
    }
}

impl_ray_packets!(Ray4, 4);

impl SoARay for Ray4 {
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
    fn set_tnear(&mut self, i: usize, near: f32) { self.tnear[i] = near; }

    fn tfar(&self, i: usize) -> f32 { self.tfar[i] }
    fn set_tfar(&mut self, i: usize, far: f32) { self.tfar[i] = far; }

    fn time(&self, i: usize) -> f32 { self.time[i] }
    fn set_time(&mut self, i: usize, time: f32) { self.time[i] = time; }

    fn mask(&self, i: usize) -> u32 { self.mask[i] }
    fn set_mask(&mut self, i: usize, mask: u32) { self.mask[i] = mask; }

    fn id(&self, i: usize) -> u32 { self.id[i] }
    fn set_id(&mut self, i: usize, id: u32) { self.id[i] = id; }

    fn flags(&self, i: usize) -> u32 { self.flags[i] }
    fn set_flags(&mut self, i: usize, flags: u32) { self.flags[i] = flags; }
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
    pub fn any_hit(&self) -> bool { self.hits().any(|h| h) }
    pub fn hits<'a>(&'a self) -> impl Iterator<Item = bool> + 'a {
        self.geomID.iter().map(|g| *g != u32::MAX)
    }
    pub fn iter(&self) -> SoAHitIter<Hit4> { SoAHitIter::new(self, 4) }
    pub fn iter_hits<'a>(&'a self) -> impl Iterator<Item = SoAHitRef<Hit4>> + 'a {
        SoAHitIter::new(self, 4).filter(|h| h.hit())
    }
}

impl SoAHit for Hit4 {
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
pub struct RayN {
    pub(crate) ptr: *mut sys::RTCRayN,
    pub(crate) len: usize,
}

impl RayN {
    pub fn org_x(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(i) }
    }

    pub fn org_y(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(self.len + i) }
    }

    pub fn org_z(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(2 * self.len + i) }
    }

    pub fn tnear(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(3 * self.len + i) }
    }

    pub fn dir_x(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(4 * self.len + i) }
    }

    pub fn dir_y(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(5 * self.len + i) }
    }

    pub fn dir_z(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(6 * self.len + i) }
    }

    pub fn time(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(7 * self.len + i) }
    }

    pub fn tfar(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(8 * self.len + i) }
    }

    pub fn mask(&self, i: usize) -> u32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const u32).add(9 * self.len + i) }
    }

    pub fn id(&self, i: usize) -> u32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const u32).add(10 * self.len + i) }
    }

    pub fn flags(&self, i: usize) -> u32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const u32).add(11 * self.len + i) }
    }
}

/// Hit packet of runtime size.
///
/// It is used to represent a packet of hits that is not known at compile
/// time, generally used as an argument to callback functions. The size
/// of the packet can only be either 1, 4, 8, or 16.
pub struct HitN {
    pub(crate) ptr: *mut sys::RTCHitN,
    pub(crate) len: usize,
}

impl HitN {
    pub fn ng_x(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(i) }
    }

    pub fn ng_y(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(self.len + i) }
    }

    pub fn ng_z(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(2 * self.len + i) }
    }

    pub fn u(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(3 * self.len + i) }
    }

    pub fn v(&self, i: usize) -> f32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const f32).add(4 * self.len + i) }
    }

    pub fn prim_id(&self, i: usize) -> u32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const u32).add(5 * self.len + i) }
    }

    pub fn geom_id(&self, i: usize) -> u32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const u32).add(6 * self.len + i) }
    }

    pub fn inst_id(&self, i: usize) -> u32 {
        debug_assert!(i < self.len, "index out of bounds");
        unsafe { *(self.ptr as *const u32).add(7 * self.len + i) }
    }
}

/// Combined ray and hit packet of runtime size.
///
/// The size of the packet can only be either 1, 4, 8, or 16.
pub struct RayHitN {
    pub(crate) ptr: *mut sys::RTCRayHitN,
    pub(crate) len: usize,
}

impl RayHitN {
    /// Returns the ray packet.
    pub fn ray_n(&self) -> RayN {
        RayN {
            ptr: self.ptr as *mut sys::RTCRayN,
            len: self.len,
        }
    }

    /// Returns the hit packet.
    pub fn hit_n(&self) -> HitN {
        HitN {
            ptr: unsafe { (self.ptr as *const u32).add(12 * self.len) as *mut sys::RTCHitN },
            len: self.len,
        }
    }
}
