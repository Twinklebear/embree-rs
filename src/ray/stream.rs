//! Ray stream types in SOA layout.

use std::{alloc, iter::Iterator, marker::PhantomData, ptr::NonNull};

use crate::{
    normalise_vector3,
    sys::{RTCHitNp, RTCRayHitNp, RTCRayNp},
    SoAHit, SoAHitIter, SoAHitRef, SoARay, SoARayIter, SoARayIterMut, INVALID_ID,
};

/// A ray stream stored in SoA format.
///
/// Each ray component is aligned to 16 bytes.
pub struct RayNp {
    /// The pointer to the start of the ray stream.
    ptr: NonNull<u8>,
    /// The number of rays in the stream.
    len: usize,
    /// The size of the allocated memory in bytes for each field of the ray
    /// stream. This is always aligned to 16 bytes.
    aligned_field_size: usize,
    marker: PhantomData<u8>,
}

impl RayNp {
    /// Allocate a new Ray stream with room for `n` rays.
    ///
    /// The rays are uninitialized.
    pub fn new(n: usize) -> RayNp {
        unsafe {
            let aligned_field_size = (n * std::mem::size_of::<f32>() + 15) & !15;
            let layout = alloc::Layout::from_size_align(aligned_field_size * 12, 16).unwrap();
            let ptr = match NonNull::new(alloc::alloc_zeroed(layout) as *mut u8) {
                Some(ptr) => ptr,
                None => alloc::handle_alloc_error(layout),
            };
            // Set the mask to 0xFFFFFFFF
            ptr.as_ptr()
                .add(aligned_field_size * 9)
                .write_bytes(0xFF, aligned_field_size);
            // Set the tfar to INFINITY
            let tfar_ptr = ptr.as_ptr().add(aligned_field_size * 8) as *mut f32;
            for i in 0..n {
                tfar_ptr.add(i).write(f32::INFINITY);
            }
            RayNp {
                ptr,
                len: n,
                aligned_field_size,
                marker: PhantomData,
            }
        }
    }

    pub fn iter(&self) -> SoARayIter<RayNp> { SoARayIter::new(self, self.len()) }

    pub fn iter_mut(&mut self) -> SoARayIterMut<RayNp> {
        let n = self.len();
        SoARayIterMut::new(self, n)
    }

    /// Returns the number of rays in the stream.
    pub fn len(&self) -> usize { self.len }

    /// Returns true if the stream is empty.
    pub fn is_empty(&self) -> bool { self.len() == 0 }

    pub fn as_raw_mut(&mut self) -> RTCRayNp {
        unsafe {
            let base_ptr = self.ptr.as_ptr();
            RTCRayNp {
                org_x: base_ptr.add(0) as *mut f32,
                org_y: base_ptr.add(self.aligned_field_size) as *mut f32,
                org_z: base_ptr.add(2 * self.aligned_field_size) as *mut f32,
                tnear: base_ptr.add(3 * self.aligned_field_size) as *mut f32,
                dir_x: base_ptr.add(4 * self.aligned_field_size) as *mut f32,
                dir_y: base_ptr.add(5 * self.aligned_field_size) as *mut f32,
                dir_z: base_ptr.add(6 * self.aligned_field_size) as *mut f32,
                time: base_ptr.add(7 * self.aligned_field_size) as *mut f32,
                tfar: base_ptr.add(8 * self.aligned_field_size) as *mut f32,
                mask: base_ptr.add(9 * self.aligned_field_size) as *mut u32,
                id: base_ptr.add(10 * self.aligned_field_size) as *mut u32,
                flags: base_ptr.add(11 * self.aligned_field_size) as *mut u32,
            }
        }
    }
}

impl Drop for RayNp {
    fn drop(&mut self) {
        unsafe {
            let layout = alloc::Layout::from_size_align(self.aligned_field_size * 12, 16).unwrap();
            alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
        }
    }
}

impl SoARay for RayNp {
    fn org(&self, i: usize) -> [f32; 3] {
        unsafe {
            let base_ptr = self.ptr.as_ptr();
            [
                *(base_ptr.add(0) as *mut f32).add(i),
                *(base_ptr.add(self.aligned_field_size) as *mut f32).add(i),
                *(base_ptr.add(2 * self.aligned_field_size) as *mut f32).add(i),
            ]
        }
    }

    fn set_org(&mut self, i: usize, o: [f32; 3]) {
        unsafe {
            let base_ptr = self.ptr.as_ptr();
            *(base_ptr.add(0) as *mut f32).add(i) = o[0];
            *(base_ptr.add(self.aligned_field_size) as *mut f32).add(i) = o[1];
            *(base_ptr.add(2 * self.aligned_field_size) as *mut f32).add(i) = o[2];
        }
    }

    fn tnear(&self, i: usize) -> f32 {
        unsafe { *(self.ptr.as_ptr().add(3 * self.aligned_field_size) as *mut f32).add(i) }
    }

    fn set_tnear(&mut self, i: usize, near: f32) {
        unsafe {
            *(self.ptr.as_ptr().add(3 * self.aligned_field_size) as *mut f32).add(i) = near;
        }
    }

    fn dir(&self, i: usize) -> [f32; 3] {
        unsafe {
            let base_ptr = self.ptr.as_ptr();
            [
                *(base_ptr.add(4 * self.aligned_field_size) as *mut f32).add(i),
                *(base_ptr.add(5 * self.aligned_field_size) as *mut f32).add(i),
                *(base_ptr.add(6 * self.aligned_field_size) as *mut f32).add(i),
            ]
        }
    }

    fn set_dir(&mut self, i: usize, d: [f32; 3]) {
        unsafe {
            let base_ptr = self.ptr.as_ptr();
            *(base_ptr.add(4 * self.aligned_field_size) as *mut f32).add(i) = d[0];
            *(base_ptr.add(5 * self.aligned_field_size) as *mut f32).add(i) = d[1];
            *(base_ptr.add(6 * self.aligned_field_size) as *mut f32).add(i) = d[2];
        }
    }

    fn time(&self, i: usize) -> f32 {
        unsafe { *(self.ptr.as_ptr().add(7 * self.aligned_field_size) as *mut f32).add(i) }
    }

    fn set_time(&mut self, i: usize, time: f32) {
        unsafe {
            *(self.ptr.as_ptr().add(7 * self.aligned_field_size) as *mut f32).add(i) = time;
        }
    }

    fn tfar(&self, i: usize) -> f32 {
        unsafe { *(self.ptr.as_ptr().add(8 * self.aligned_field_size) as *mut f32).add(i) }
    }

    fn set_tfar(&mut self, i: usize, far: f32) {
        unsafe {
            *(self.ptr.as_ptr().add(8 * self.aligned_field_size) as *mut f32).add(i) = far;
        }
    }

    fn mask(&self, i: usize) -> u32 {
        unsafe { *(self.ptr.as_ptr().add(9 * self.aligned_field_size) as *mut u32).add(i) }
    }

    fn set_mask(&mut self, i: usize, mask: u32) {
        unsafe {
            *(self.ptr.as_ptr().add(9 * self.aligned_field_size) as *mut u32).add(i) = mask;
        }
    }

    fn id(&self, i: usize) -> u32 {
        unsafe { *(self.ptr.as_ptr().add(10 * self.aligned_field_size) as *mut u32).add(i) }
    }

    fn set_id(&mut self, i: usize, id: u32) {
        unsafe {
            *(self.ptr.as_ptr().add(10 * self.aligned_field_size) as *mut u32).add(i) = id;
        }
    }

    fn flags(&self, i: usize) -> u32 {
        unsafe { *(self.ptr.as_ptr().add(11 * self.aligned_field_size) as *mut u32).add(i) }
    }

    fn set_flags(&mut self, i: usize, flags: u32) {
        unsafe {
            *(self.ptr.as_ptr().add(11 * self.aligned_field_size) as *mut u32).add(i) = flags;
        }
    }
}

#[test]
fn test_stream_layout_raynp() {
    let mut ray0 = RayNp::new(11);
    assert_eq!(ray0.aligned_field_size, 48);

    let ray1 = RayNp::new(17);
    assert_eq!(ray1.aligned_field_size, 80);

    assert_eq!(
        std::mem::size_of::<RayNp>(),
        24,
        concat!("Size of: ", stringify!(RayNp))
    );

    assert_eq!(ray0.as_raw_mut().org_x as usize % 16, 0);
    assert_eq!(ray0.as_raw_mut().org_y as usize % 16, 0);
    assert_eq!(ray0.as_raw_mut().org_z as usize % 16, 0);
    assert_eq!(ray0.as_raw_mut().tnear as usize % 16, 0);
    assert_eq!(ray0.as_raw_mut().dir_x as usize % 16, 0);
    assert_eq!(ray0.as_raw_mut().dir_y as usize % 16, 0);
    assert_eq!(ray0.as_raw_mut().dir_z as usize % 16, 0);
    assert_eq!(ray0.as_raw_mut().time as usize % 16, 0);
    assert_eq!(ray0.as_raw_mut().tfar as usize % 16, 0);
    assert_eq!(ray0.as_raw_mut().mask as usize % 16, 0);
    assert_eq!(ray0.as_raw_mut().id as usize % 16, 0);
    assert_eq!(ray0.as_raw_mut().flags as usize % 16, 0);
}

#[test]
fn test_stream_new_raynp() {
    let ray = RayNp::new(135);
    for i in 0..135 {
        assert_eq!(ray.org(i), [0.0, 0.0, 0.0]);
        assert_eq!(ray.dir(i), [0.0, 0.0, 0.0]);
        assert_eq!(ray.tnear(i), 0.0);
        assert_eq!(ray.tfar(i), f32::INFINITY);
        assert_eq!(ray.mask(i), 0xFFFFFFFF);
        assert_eq!(ray.id(i), 0);
        assert_eq!(ray.flags(i), 0);
    }
}

/// A hit stream in SoA format.
///
/// Each hit component is aligned to 16 bytes.
pub struct HitNp {
    /// The pointer to the data.
    ptr: NonNull<u8>,
    /// The number of hits.
    len: usize,
    /// The size of each field, rounded up to the nearest multiple of 16.
    aligned_field_size: usize,
    marker: PhantomData<u8>,
}

impl HitNp {
    pub fn new(n: usize) -> HitNp {
        unsafe {
            let aligned_field_size = (std::mem::size_of::<f32>() * n + 15) & !15;
            let layout = alloc::Layout::from_size_align(aligned_field_size * 8, 16).unwrap();
            let ptr = match NonNull::new(alloc::alloc_zeroed(layout) as *mut u8) {
                Some(ptr) => ptr,
                None => alloc::handle_alloc_error(layout),
            };
            // Set the primID, geomID, instID to INVALID_ID.
            (ptr.as_ptr() as *mut u8)
                .add(5 * aligned_field_size)
                .write_bytes(0xFF, aligned_field_size * 3);
            HitNp {
                ptr,
                len: n,
                aligned_field_size,
                marker: PhantomData,
            }
        }
    }

    pub fn any_hit(&self) -> bool { self.iter_validity().any(|g| g) }

    pub fn iter_validity(&self) -> impl Iterator<Item = bool> + '_ {
        unsafe {
            std::slice::from_raw_parts(
                self.ptr.as_ptr().add(6 * self.aligned_field_size) as *const u32,
                self.len,
            )
            .iter()
            .map(|g| *g != INVALID_ID)
        }
    }

    pub fn iter(&self) -> SoAHitIter<HitNp> { SoAHitIter::new(self, self.len()) }

    pub fn iter_hits(&self) -> impl Iterator<Item = SoAHitRef<HitNp>> {
        SoAHitIter::new(self, self.len()).filter(|h| h.is_valid())
    }

    pub fn len(&self) -> usize { self.len }

    pub fn is_empty(&self) -> bool { self.len == 0 }

    pub fn as_raw_mut(&mut self) -> RTCHitNp {
        unsafe {
            let base_ptr = self.ptr.as_ptr();
            RTCHitNp {
                Ng_x: base_ptr.add(0) as *mut f32,
                Ng_y: base_ptr.add(self.aligned_field_size) as *mut f32,
                Ng_z: base_ptr.add(2 * self.aligned_field_size) as *mut f32,
                u: base_ptr.add(3 * self.aligned_field_size) as *mut f32,
                v: base_ptr.add(4 * self.aligned_field_size) as *mut f32,
                primID: base_ptr.add(5 * self.aligned_field_size) as *mut u32,
                geomID: base_ptr.add(6 * self.aligned_field_size) as *mut u32,
                instID: [base_ptr.add(7 * self.aligned_field_size) as *mut u32],
            }
        }
    }
}

impl Drop for HitNp {
    fn drop(&mut self) {
        unsafe {
            let layout = alloc::Layout::from_size_align(self.aligned_field_size * 8, 16).unwrap();
            alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
        }
    }
}

impl SoAHit for HitNp {
    fn normal(&self, i: usize) -> [f32; 3] {
        unsafe {
            let base_ptr = self.ptr.as_ptr();
            [
                *(base_ptr.add(0) as *mut f32).add(i),
                *(base_ptr.add(self.aligned_field_size) as *mut f32).add(i),
                *(base_ptr.add(2 * self.aligned_field_size) as *mut f32).add(i),
            ]
        }
    }

    fn unit_normal(&self, i: usize) -> [f32; 3] { normalise_vector3(self.normal(i)) }

    fn set_normal(&mut self, i: usize, n: [f32; 3]) {
        unsafe {
            let base_ptr = self.ptr.as_ptr();
            *(base_ptr.add(0) as *mut f32).add(i) = n[0];
            *(base_ptr.add(self.aligned_field_size) as *mut f32).add(i) = n[1];
            *(base_ptr.add(2 * self.aligned_field_size) as *mut f32).add(i) = n[2];
        }
    }

    fn u(&self, i: usize) -> f32 {
        unsafe { *(self.ptr.as_ptr().add(3 * self.aligned_field_size) as *mut f32).add(i) }
    }

    fn v(&self, i: usize) -> f32 {
        unsafe { *(self.ptr.as_ptr().add(4 * self.aligned_field_size) as *mut f32).add(i) }
    }

    fn uv(&self, i: usize) -> [f32; 2] {
        unsafe {
            let base_ptr = self.ptr.as_ptr();
            [
                *(base_ptr.add(3 * self.aligned_field_size) as *mut f32).add(i),
                *(base_ptr.add(4 * self.aligned_field_size) as *mut f32).add(i),
            ]
        }
    }

    fn set_u(&mut self, i: usize, u: f32) {
        unsafe {
            *(self.ptr.as_ptr().add(3 * self.aligned_field_size) as *mut f32).add(i) = u;
        }
    }

    fn set_v(&mut self, i: usize, v: f32) {
        unsafe {
            *(self.ptr.as_ptr().add(4 * self.aligned_field_size) as *mut f32).add(i) = v;
        }
    }

    fn set_uv(&mut self, i: usize, uv: [f32; 2]) {
        unsafe {
            let base_ptr = self.ptr.as_ptr();
            *(base_ptr.add(3 * self.aligned_field_size) as *mut f32).add(i) = uv[0];
            *(base_ptr.add(4 * self.aligned_field_size) as *mut f32).add(i) = uv[1];
        }
    }

    fn prim_id(&self, i: usize) -> u32 {
        unsafe { *(self.ptr.as_ptr().add(5 * self.aligned_field_size) as *mut u32).add(i) }
    }

    fn set_prim_id(&mut self, i: usize, id: u32) {
        unsafe {
            *(self.ptr.as_ptr().add(5 * self.aligned_field_size) as *mut u32).add(i) = id;
        }
    }

    fn geom_id(&self, i: usize) -> u32 {
        unsafe { *(self.ptr.as_ptr().add(6 * self.aligned_field_size) as *mut u32).add(i) }
    }

    fn set_geom_id(&mut self, i: usize, id: u32) {
        unsafe {
            *(self.ptr.as_ptr().add(6 * self.aligned_field_size) as *mut u32).add(i) = id;
        }
    }

    fn inst_id(&self, i: usize) -> u32 {
        unsafe { *(self.ptr.as_ptr().add(7 * self.aligned_field_size) as *mut u32).add(i) }
    }

    fn set_inst_id(&mut self, i: usize, id: u32) {
        unsafe {
            *(self.ptr.as_ptr().add(7 * self.aligned_field_size) as *mut u32).add(i) = id;
        }
    }
}

#[test]
fn test_stream_layout_hitnp() {
    let mut hit0 = HitNp::new(9);
    assert_eq!(hit0.aligned_field_size, 48);

    let hit1 = HitNp::new(18);
    assert_eq!(hit1.aligned_field_size, 80);

    assert_eq!(
        std::mem::size_of::<HitNp>(),
        24,
        concat!("Size of: ", stringify!(RayNp))
    );

    assert_eq!(hit0.as_raw_mut().Ng_x as usize % 16, 0);
    assert_eq!(hit0.as_raw_mut().Ng_y as usize % 16, 0);
    assert_eq!(hit0.as_raw_mut().Ng_z as usize % 16, 0);
    assert_eq!(hit0.as_raw_mut().u as usize % 16, 0);
    assert_eq!(hit0.as_raw_mut().v as usize % 16, 0);
    assert_eq!(hit0.as_raw_mut().primID as usize % 16, 0);
    assert_eq!(hit0.as_raw_mut().geomID as usize % 16, 0);
    assert_eq!(hit0.as_raw_mut().instID[0] as usize % 16, 0);
}

#[test]
fn test_stream_new_hitnp() {
    let mut hit = HitNp::new(13);
    for hit in hit.iter_hits() {
        assert_eq!(hit.normal(), [0.0, 0.0, 0.0]);
        assert_eq!(hit.uv(), [0.0, 0.0]);
        assert_eq!(hit.prim_id(), INVALID_ID);
        assert_eq!(hit.geom_id(), INVALID_ID);
        assert_eq!(hit.inst_id(), INVALID_ID);
    }
}

pub struct RayHitNp {
    pub ray: RayNp,
    pub hit: HitNp,
}

impl RayHitNp {
    pub fn new(ray: RayNp) -> RayHitNp {
        let n = ray.len();
        RayHitNp {
            ray,
            hit: HitNp::new(n),
        }
    }

    pub fn iter(&self) -> std::iter::Zip<SoARayIter<RayNp>, SoAHitIter<HitNp>> {
        self.ray.iter().zip(self.hit.iter())
    }
    pub fn len(&self) -> usize { self.ray.len() }

    pub fn is_empty(&self) -> bool { self.len() == 0 }

    pub fn as_raw(&mut self) -> RTCRayHitNp {
        RTCRayHitNp {
            ray: self.ray.as_raw_mut(),
            hit: self.hit.as_raw_mut(),
        }
    }
}
