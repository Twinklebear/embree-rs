use crate::{sys, INVALID_ID};
use std::fmt::{Debug, Formatter};

mod packet;
mod soa;
mod stream;

pub use packet::*;
pub use soa::*;
pub use stream::*;

/// Trait for types that can be converted to a [`Ray`].
///
/// Embree uses poor man's inheritance to make it possible to extend the [`Ray`]
/// and [`RayHit`] types with additional data. This trait provides a way to
/// convert between the base types and the extended types. See also the
/// [`AsRayHit`]
///
/// # Safety
///
/// Structs that implement this trait must guarantee that they are
/// layout-compatible with [`Ray`] (i.e. pointer casts between the two types are
/// valid). The corresponding pattern in C is called poor man's inheritance. See
/// [`RayExt`] for an example of how to do this.
pub unsafe trait AsRay: Sized {
    type RayExt: Sized;

    fn as_ray(&self) -> &Ray;
    fn as_ray_mut(&mut self) -> &mut Ray;
    fn as_ray_ext(&self) -> Option<&Self::RayExt>;
    fn as_ray_ext_mut(&mut self) -> Option<&mut Self::RayExt>;

    fn as_ray_ptr(&self) -> *const Ray { self.as_ray() as *const Ray }
    fn as_ray_mut_ptr(&mut self) -> *mut Ray { self.as_ray_mut() as *mut Ray }
}

/// Trait for types that can be converted to a [`Ray`].
///
/// Embree uses poor man's inheritance to make it possible to extend the [`Ray`]
/// and [`RayHit`] types with additional data. This trait provides a way to
/// convert between the base types and the extended types. See also the
/// [`AsRay`]
///
/// # Safety
///
/// Structs that implement this trait must guarantee that they are layout
/// compatible with [`Ray`] (i.e. pointer casts between the two types are
/// valid).
pub unsafe trait AsRayHit: AsRay {
    type RayHitExt: Sized;

    fn as_ray_hit(&self) -> &RayHit;
    fn as_ray_hit_mut(&mut self) -> &mut RayHit;
    fn as_ray_hit_ext(&self) -> Option<&Self::RayHitExt>;
    fn as_ray_hit_ext_mut(&mut self) -> Option<&mut Self::RayHitExt>;

    fn as_ray_hit_ptr(&self) -> *const RayHit { self.as_ray_hit() as *const RayHit }
    fn as_ray_hit_mut_ptr(&mut self) -> *mut RayHit { self.as_ray_hit_mut() as *mut RayHit }
}

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
/// and size 16 (RTCRay16 type). A const-generic type [`RayNt`] is
/// defined for ray packets of arbitrary size N at compile time.
///
/// See [`sys::RTCRay`] for more details.
pub type Ray = sys::RTCRay;

impl Ray {
    /// Creates a new ray starting at `origin` and heading in direction `dir`
    pub fn new(origin: [f32; 3], direction: [f32; 3]) -> Ray {
        Ray::segment(origin, direction, 0.0, f32::INFINITY)
    }

    pub fn new_with_id(origin: [f32; 3], direction: [f32; 3], id: u32) -> Ray {
        Ray {
            org_x: origin[0],
            org_y: origin[1],
            org_z: origin[2],
            tnear: 0.0,
            dir_x: direction[0],
            dir_y: direction[1],
            dir_z: direction[2],
            tfar: f32::INFINITY,
            time: 0.0,
            mask: u32::MAX,
            id,
            flags: 0,
        }
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

unsafe impl AsRay for Ray {
    type RayExt = ();

    fn as_ray(&self) -> &Ray { self }
    fn as_ray_mut(&mut self) -> &mut Ray { self }
    fn as_ray_ext(&self) -> Option<&()> { None }
    fn as_ray_ext_mut(&mut self) -> Option<&mut ()> { None }
}

/// Extended ray type that contains an additional data field.
///
/// For the reason that the ray passed to the filter callback functions
/// and user geometry callback functions is guaranteed to be the same
/// ray pointer initially provided to the ray query function by the user,
/// it is SAFE to extend the ray by additional data and access this data
/// inside the filter callback functions (e.g. to accumulate opacity) and
/// user geometry callback functions (e.g. to accumulate color).
///
/// To make sure that the extended ray type is layout-compatible with
/// the ray type, the additional data field must be `#[repr(C)]`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RayExt<E: Sized + Clone + Copy> {
    pub ray: Ray,
    pub ext: E,
}

impl<E> Debug for RayExt<E>
where
    E: Debug + Sized + Clone + Copy,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RayExt")
            .field("ray", &self.ray)
            .field("ext", &self.ext)
            .finish()
    }
}

impl<E> RayExt<E>
where
    E: Sized + Copy + Clone,
{
    pub fn new(ray: Ray, ext: E) -> Self { Self { ray, ext } }
}

#[test]
fn test_ray_ext_pointer_compatability_with_ray_pointer() {
    let ray = RayExt {
        ray: Ray::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
        ext: 10u32,
    };
    assert_eq!(
        &ray as *const RayExt<u32> as *const Ray,
        &ray.ray as *const Ray
    );

    #[repr(C)]
    #[derive(Clone, Copy, Debug)]
    struct Extra {
        a: u32,
        b: u32,
        c: f32,
        d: f32,
    }

    let mut ray2 = RayExt::<Extra> {
        ray: Ray::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
        ext: Extra {
            a: 1,
            b: 2,
            c: 3.0,
            d: 4.0,
        },
    };

    let ray_ptr = &mut ray2 as *mut RayExt<Extra> as *mut Ray;
    let ray_ext = unsafe { &mut *(ray_ptr as *mut RayExt<Extra>) };
    ray_ext.ext.a = 10;
    ray_ext.ext.d = 40.0;

    assert_eq!(ray2.ext.a, 10);
    assert_eq!(ray2.ext.d, 40.0);
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
/// [`HitNt`] defines the type for hit packets of arbitrary size N at
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
/// type), and size 16 (RTCRayHit16 type). A const-generic type [`RayHitNt`]
/// is defined for ray/hit packets of arbitrary size N at compile time.
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

impl From<Ray> for RayHit {
    fn from(value: Ray) -> Self { RayHit::from_ray(value) }
}

unsafe impl AsRay for RayHit {
    type RayExt = Hit;

    fn as_ray(&self) -> &Ray { &self.ray }

    fn as_ray_mut(&mut self) -> &mut Ray { &mut self.ray }

    fn as_ray_ext(&self) -> Option<&Self::RayExt> { Some(&self.hit) }

    fn as_ray_ext_mut(&mut self) -> Option<&mut Self::RayExt> { Some(&mut self.hit) }
}

unsafe impl AsRayHit for RayHit {
    type RayHitExt = ();

    fn as_ray_hit(&self) -> &RayHit { self }

    fn as_ray_hit_mut(&mut self) -> &mut RayHit { self }

    fn as_ray_hit_ext(&self) -> Option<&Self::RayHitExt> { None }

    fn as_ray_hit_ext_mut(&mut self) -> Option<&mut Self::RayHitExt> { None }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RayHitAsRayExtra<E: Sized + Copy + Clone> {
    pub hit: Hit,
    pub ext: E,
}

impl<E> Debug for RayHitAsRayExtra<E>
where
    E: Debug + Sized + Copy + Clone,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RayHitExtra")
            .field("hit", &self.hit)
            .field("ext", &self.ext)
            .finish()
    }
}

/// Extended ray type that contains an additional data field.
///
/// To make sure that the extended ray type is layout-compatible with
/// the ray type, the additional data field must be `#[repr(C)]`.
#[repr(C)]
#[repr(align(16))]
#[derive(Clone, Copy)]
pub struct RayHitExt<E: Sized + Clone + Copy> {
    pub ray: RayHit,
    pub ext: E,
}

impl<E> Debug for RayHitExt<E>
where
    E: Debug + Sized + Clone + Copy,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RayHitExt")
            .field("ray", &self.ray)
            .field("ext", &self.ext)
            .finish()
    }
}

impl<E> RayHitExt<E>
where
    E: Sized + Clone + Copy,
{
    pub fn new(ray_hit: RayHit, ext: E) -> Self { Self { ray: ray_hit, ext } }

    pub fn new_with_ray(ray: Ray, ext: E) -> Self {
        Self {
            ray: RayHit::from_ray(ray),
            ext,
        }
    }
}

unsafe impl<E> AsRay for RayHitExt<E>
where
    E: Sized + Clone + Copy,
{
    type RayExt = RayHitAsRayExtra<E>;

    fn as_ray(&self) -> &Ray { &self.ray.ray }

    fn as_ray_mut(&mut self) -> &mut Ray { &mut self.ray.ray }

    fn as_ray_ext(&self) -> Option<&Self::RayExt> {
        Some(unsafe { &*(&self.ray.hit as *const Hit as *const Self::RayExt) })
    }

    fn as_ray_ext_mut(&mut self) -> Option<&mut Self::RayExt> {
        Some(unsafe { &mut *(&mut self.ray.hit as *mut Hit as *mut Self::RayExt) })
    }
}

unsafe impl<E> AsRayHit for RayHitExt<E>
where
    E: Sized + Copy + Clone,
{
    type RayHitExt = E;

    fn as_ray_hit(&self) -> &RayHit { &self.ray }

    fn as_ray_hit_mut(&mut self) -> &mut RayHit { &mut self.ray }

    fn as_ray_hit_ext(&self) -> Option<&Self::RayHitExt> { Some(&self.ext) }

    fn as_ray_hit_ext_mut(&mut self) -> Option<&mut Self::RayHitExt> { Some(&mut self.ext) }
}

#[test]
fn test_ray_hit_ext_pointer_compatability_with_ray_hit() {
    let ray = RayHitExt {
        ray: RayHit::from_ray(Ray::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0])),
        ext: [0.0f32, 0.0, 0.0, 0.0],
    };
    assert_eq!(
        &ray as *const RayHitExt<[f32; 4]> as *const RayHit,
        &ray.ray as *const RayHit
    );

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Extra {
        a: i32,
        b: u64,
        c: f32,
        d: [u8; 4],
    }

    let mut ray = RayHitExt::<Extra> {
        ray: RayHit::from_ray(Ray::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0])),
        ext: Extra {
            a: 1,
            b: 2,
            c: 3.0,
            d: [4, 5, 6, 7],
        },
    };

    let ray_ptr = &mut ray as *mut RayHitExt<Extra> as *mut RayHit;
    let ray_ext = unsafe { &mut *(ray_ptr as *mut RayHitExt<Extra>) };
    ray_ext.ext.a = 10;
    ray_ext.ext.d = [30, 31, 32, 33];

    assert_eq!(ray.ext.a, 10);
    assert_eq!(ray.ext.d, [30, 31, 32, 33]);

    ray.as_ray_hit_ext_mut().unwrap().a = 20;

    assert_eq!(ray.ext.a, 20);
}

#[test]
fn test_ray_hit_ext_pointer_compatability_with_ray() {
    let ray = RayHitExt {
        ray: RayHit::from_ray(Ray::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0])),
        ext: [0.0f32, 0.0, 0.0, 0.0],
    };
    assert_eq!(
        &ray as *const RayHitExt<[f32; 4]> as *const RayHit,
        &ray.ray as *const RayHit
    );

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Extra {
        a: i32,
        b: u64,
        d: [u8; 4],
    }

    let mut ray = RayHitExt::<Extra> {
        ray: RayHit::from_ray(Ray::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0])),
        ext: Extra {
            a: 1,
            b: 2,
            d: [4, 5, 6, 7],
        },
    };

    ray.as_ray_mut().org_x = 10.0;
    ray.as_ray_mut().dir_x = 20.0;

    match ray.as_ray_ext_mut() {
        None => {}
        Some(ray_ext) => {
            ray_ext.hit.geomID = 30;
            ray_ext.hit.primID = 40;
            ray_ext.ext.d = [50, 51, 52, 53];
        }
    }

    assert_eq!(ray.ray.ray.org(), [10.0, 0.0, 0.0]);
    assert_eq!(ray.ray.ray.dir(), [20.0, 0.0, 0.0]);
    assert_eq!(ray.ext.d, [50, 51, 52, 53]);
    assert_eq!(ray.ray.hit.geomID, 30);
    assert_eq!(ray.ray.hit.primID, 40);
}
