use crate::{sys, INVALID_ID};

pub mod packet;
pub mod soa;
pub mod stream;

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
/// The ray segment must be in the range [0, \inf], thus ranges start
/// behind the ray origin are not allowed, but ranges can reach to infinity.
/// For rays inside a ray stream, `tfar` < `tnear` identifies an *inactive*
/// ray.
///
/// Ray identifiers are used to identify rays inside a callback function,
/// event if the order of rays inside a ray packet or stream has changed.
///
/// [`packet`](`crate::ray::packet`) defines types in SOA (structure of array)
/// layout for ray packets of size 4 (RTCRay4 type), size 8 (RTCRay8 type),
/// and size 16 (RTCRay16 type). A const-generic type [`RayPacket`] is
/// defined for ray packets of arbitrary size N at compile time.
///
/// See [`sys::RTCRay`] for more details.
pub type Ray = sys::RTCRay;

impl Ray {
    /// Creates a new ray starting at `origin` and heading in direction `dir`
    pub fn new(origin: [f32; 3], direction: [f32; 3]) -> Ray {
        Ray::segment(origin, direction, 0.0, f32::INFINITY)
    }

    /// Creates a new ray starting at `origin` and heading in direction `dir`
    /// with a segment starting at `tnear` and ending at `tfar`.
    pub fn segment(origin: [f32; 3], direction: [f32; 3], tnear: f32, tfar: f32) -> Ray {
        Ray {
            org_x: origin[0],
            org_y: origin[1],
            org_z: origin[2],
            tnear,
            dir_x: direction[0],
            dir_y: direction[1],
            dir_z: direction[2],
            tfar,
            time: 0.0,
            mask: u32::MAX,
            id: 0,
            flags: 0,
        }
    }

    /// Returns the origin of the ray.
    pub fn org(&self) -> [f32; 3] { [self.org_x, self.org_y, self.org_z] }

    /// Returns the direction (un-normalized) of the ray.
    pub fn dir(&self) -> [f32; 3] { [self.dir_x, self.dir_y, self.dir_z] }

    /// Returns the normalized direction of the ray.
    ///
    /// The direction is normalized by dividing it by its length, which
    /// may produce a NaN if the direction is zero.
    pub fn dir_normalized(&self) -> [f32; 3] {
        let dir = self.dir();
        let len = dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2];
        let len = len.sqrt();
        [dir[0] / len, dir[1] / len, dir[2] / len]
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
/// ([`geomID`](([`sys::RTCHit::geomID`]) member), and instance ID
/// stack ([`instID`]([`sys::RTCHit::instID`]) member) of the hit.
/// The parametric intersection distance is not stored inside the hit,
/// but stored inside the [`tfar`]([`sys::RTCRay::tfar`]) member of the ray.
///
/// There exists structures in SOA (structure of array) layout for hit packets
/// of size 4 (RTCHit4 type), size 8 (RTCHit8 type), and size 16 (RTCHit16
/// type).
///
/// [`HitPacket`] defines the type for hit packets of arbitrary size N at
/// compile time.
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
    pub fn normal_normalized(&self) -> [f32; 3] {
        let normal = self.normal();
        let len = normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2];
        let len = len.sqrt();
        [normal[0] / len, normal[1] / len, normal[2] / len]
    }

    /// Returns the barycentric u/v coordinates of the hit.
    pub fn barycentric(&self) -> [f32; 2] { [self.u, self.v] }
}

/// New type alias for [`sys::RTCRayHit`] that provides some convenience
/// methods.
///
/// A combined single ray/hit structure. This structure is used as input
/// for the `intersect-type` functions and stores the ray to intersect
/// and some hit fields that hold the intersection result afterwards.
///
/// [`packet`](`crate::ray::packet`) defines types in SOA (structure of array)
/// layout for ray/hit packets of size 4 (RTCRayHit4 type), size 8 (RTCRayHit8
/// type), and size 16 (RTCRayHit16 type). A const-generic type [`RayHitPacket`]
/// is defined for ray/hit packets of arbitrary size N at compile time.
///
/// See [`sys::RTCRayHit`] for more details.
pub type RayHit = sys::RTCRayHit;

impl RayHit {
    /// Creates a new [`RayHit`] ready to be used in an intersection query.
    pub fn new(ray: Ray) -> RayHit {
        RayHit {
            ray,
            hit: Hit::default(),
        }
    }

    /// Returns true if the hit is valid (i.e. the ray hit something).
    pub fn is_valid(&self) -> bool { self.hit.geomID != INVALID_ID }

    /// Calculates the hit point from the ray and the hit distance.
    pub fn hit_point(&self) -> [f32; 3] {
        let t = self.ray.tfar;
        [
            self.ray.org_x + self.ray.dir_x * t,
            self.ray.org_y + self.ray.dir_y * t,
            self.ray.org_z + self.ray.dir_z * t,
        ]
    }
}
