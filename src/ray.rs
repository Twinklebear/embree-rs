use crate::{normalise_vector3, sys, INVALID_ID};

pub mod packet;
mod soa;
pub mod stream;

pub use packet::*;
pub use soa::*;
pub use stream::*;

/// New type alias for [`sys::RTCRay`] that provides some convenience
/// methods.
///
/// The ray contains the origin ([`org_x`](`sys::RTCRay::org_x`),
/// [`org_y`](`sys::RTCRay::org_y`), [`org_z`](`sys::RTCRay::org_z`) members),
/// the direction vector ([`dir_x`](`sys::RTCRay::dir_x`),
/// [`dir_y`](`sys::RTCRay::dir_y`), [`dir_z`](`sys::RTCRay::dir_z`) members),
/// the ray segment ([`tnear`](`sys::RTCRay::tnear`) and
/// [`tfar`](`sys::RTCRay::tfar`) members). The ray direction does NOT need
/// to be normalized, and only the parameter range specified  by
/// [`tnear`](`sys::RTCRay::tnear`) and [`tfar`](`sys::RTCRay::tfar`) is
/// considered valid.
///
/// The ray segment must be in the range \[0, \inf\], thus ranges start
/// behind the ray origin are not allowed, but ranges can reach to infinity.
/// For rays inside a ray stream, `tfar` < `tnear` identifies an *inactive*
/// ray.
///
/// Ray identifiers are used to identify rays inside a callback function,
/// event if the order of rays inside a ray packet or stream has changed.
///
/// [`packet`](`crate::ray::packet`) defines types in SOA (structure of array)
/// layout for ray packets of size 4 ([`Ray4`]), size 8 ([`Ray8`]),
/// and size 16 ([`Ray16`]).
///
/// See [`sys::RTCRay`] for more details.
pub type Ray = sys::RTCRay;

impl Ray {
    /// Creates a new ray.
    ///
    /// Basic constructor that initializes all fields of the ray.
    pub fn new(
        origin: [f32; 3],
        direction: [f32; 3],
        near: f32,
        far: f32,
        time: f32,
        mask: u32,
        id: u32,
    ) -> Ray {
        Ray {
            org_x: origin[0],
            org_y: origin[1],
            org_z: origin[2],
            tnear: near,
            dir_x: direction[0],
            dir_y: direction[1],
            dir_z: direction[2],
            tfar: far,
            time,
            mask,
            id,
            flags: 0,
        }
    }

    /// Creates a new ray segment.
    ///
    /// The ray starting at `origin` and heading in direction `dir`
    /// with a segment starting at `tnear` and ending at `tfar`.
    pub fn segment(origin: [f32; 3], direction: [f32; 3], tnear: f32, tfar: f32) -> Ray {
        Self::new(origin, direction, tnear, tfar, 0.0, u32::MAX, 0)
    }

    /// Creates a new segment of ray with an ID.
    pub fn segment_with_id(
        origin: [f32; 3],
        direction: [f32; 3],
        tnear: f32,
        tfar: f32,
        id: u32,
    ) -> Ray {
        Self::new(origin, direction, tnear, tfar, 0.0, u32::MAX, id)
    }

    /// Returns the origin of the ray.
    pub fn org(&self) -> [f32; 3] { [self.org_x, self.org_y, self.org_z] }

    /// Returns the direction (un-normalized) of the ray.
    pub fn dir(&self) -> [f32; 3] { [self.dir_x, self.dir_y, self.dir_z] }

    /// Returns the normalized direction of the ray.
    ///
    /// Do not use this method to calculate the hit point, use [`dir`] instead.
    pub fn unit_dir(&self) -> [f32; 3] { normalise_vector3(self.dir()) }

    /// Calculates the hit point from the ray and the hit distance.
    pub fn hit_point(&self) -> [f32; 3] {
        let t = self.tfar;
        [
            self.org_x + self.dir_x * t,
            self.org_y + self.dir_y * t,
            self.org_z + self.dir_z * t,
        ]
    }
}

impl Default for Ray {
    fn default() -> Self {
        Ray {
            org_x: 0.0,
            org_y: 0.0,
            org_z: 0.0,
            tnear: 0.0,
            dir_x: 0.0,
            dir_y: 0.0,
            dir_z: 0.0,
            tfar: f32::INFINITY,
            time: 0.0,
            mask: u32::MAX,
            id: 0,
            flags: 0,
        }
    }
}

/// New type alias for [`sys::RTCHit`] that provides some convenience
/// methods.
///
/// [`Hit`] defines the type of a ray/primitive intersection result. The
/// hit contains the un-normalized geometric normal in object space at the
/// hit location ([`Ng_x`]([`sys::RTCHit::Ng_x`]),
/// [`Ng_y`]([`sys::RTCHit::Ng_y`]), [`Ng_z`]([`sys::RTCHit::Ng_z`]) members),
/// the barycentric u/v coordinates of the hit ([`u`]([`sys::RTCHit::u`]) and
/// [`v`]([`sys::RTCHit::v`]) members), as well as the primitive ID
/// ([`primID`]([`sys::RTCHit::primID`]) member), geometry ID
/// (`geomID`, [`sys::RTCHit::geomID`] member), and instance ID
/// stack (`instID`, [`sys::RTCHit::instID`] member) of the hit.
/// The parametric intersection distance is not stored inside the hit,
/// but stored inside the `tfar`([`sys::RTCRay::tfar`]) member of the ray.
///
/// There exists structures in SOA (structure of array) layout for hit packets
/// of size 4 ([`Hit4`]), size 8 ([`Hit8`]), and size 16 ([`Hit16`]).
///
/// See [`sys::RTCHit`] for more details.
pub type Hit = sys::RTCHit;

impl Default for Hit {
    fn default() -> Self {
        Hit {
            Ng_x: 0.0,
            Ng_y: 0.0,
            Ng_z: 0.0,
            u: 0.0,
            v: 0.0,
            primID: INVALID_ID,
            geomID: INVALID_ID,
            instID: [INVALID_ID; 1],
        }
    }
}

impl Hit {
    /// Returns the normal at the hit point (un-normalized).
    pub fn normal(&self) -> [f32; 3] { [self.Ng_x, self.Ng_y, self.Ng_z] }

    /// Returns the normalized normal at the hit point.
    pub fn unit_normal(&self) -> [f32; 3] { normalise_vector3(self.normal()) }

    /// Returns the barycentric u/v coordinates of the hit.
    pub fn barycentric(&self) -> [f32; 2] { [self.u, self.v] }

    /// Returns if the hit is valid, i.e. the ray hit something.
    pub fn is_valid(&self) -> bool { self.geomID != INVALID_ID }
}

/// New type alias for [`sys::RTCRayHit`] that provides some convenience
/// methods.
///
/// A combined single ray/hit structure. This structure is used as input
/// for the `intersect-type` functions and stores the ray to intersect
/// and some hit fields that hold the intersection result afterwards.
///
/// [`packet`](`crate::ray::packet`) defines types in SOA (structure of array)
/// layout for ray/hit packets of size 4 [`RayHit4`], size 8 [`RayHit8`], and
/// size 16 [`RayHit16`].
///
/// See [`sys::RTCRayHit`] for more details.
pub type RayHit = sys::RTCRayHit;

impl RayHit {
    /// Creates a new [`RayHit`] ready to be used in an intersection query.
    pub fn from_ray(ray: Ray) -> RayHit {
        RayHit {
            ray,
            hit: Hit::default(),
        }
    }
}

impl Default for RayHit {
    fn default() -> Self {
        RayHit {
            ray: Ray::default(),
            hit: Hit::default(),
        }
    }
}

impl From<Ray> for RayHit {
    fn from(value: Ray) -> Self { RayHit::from_ray(value) }
}
