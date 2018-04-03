use std::marker::PhantomData;

use sys::*;
use device::Device;
use triangle_mesh::TriangleMesh;
use ray::{Ray, RayHit, IntersectContext};

pub struct Scene<'a> {
    pub(crate) handle: RTCScene,
    /// We don't need to actually keep a reference to the device,
    /// we just need to track its lifetime for correctness
    device: PhantomData<&'a Device>,
}
impl<'a> Scene<'a> {
    pub fn new(device: &'a Device) -> Scene {
        Scene {
            handle: unsafe { rtcNewScene(device.handle) },
            device: PhantomData
        }
    }
    pub fn attach_geometry(&mut self, mesh: &TriangleMesh) -> u32 {
        unsafe { rtcAttachGeometry(self.handle, mesh.handle) }
    }
    pub fn commit(&self) {
        unsafe { rtcCommitScene(self.handle); }
    }
    pub fn intersect(&self, ctx: &mut IntersectContext, ray: &mut RayHit) {
        unsafe {
            rtcIntersect1(self.handle,
                          ctx as *mut RTCIntersectContext,
                          ray as *mut RTCRayHit);
        }
    }
    pub fn occluded(&self, ctx: &mut IntersectContext, ray: &mut Ray) {
        unsafe { 
            rtcOccluded1(self.handle,
                         ctx as *mut RTCIntersectContext,
                         ray as *mut RTCRay);
        }
    }
}

impl<'a> Drop for Scene<'a> {
    fn drop(&mut self) {
        unsafe { rtcReleaseScene(self.handle); }
    }
}

