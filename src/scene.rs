use std::collections::HashMap;
use std::marker::PhantomData;

use device::Device;
use geometry::Geometry;
use ray::{IntersectContext, Ray, RayHit};
use sys::*;

pub struct Scene<'a> {
    pub(crate) handle: RTCScene,
    /// We don't need to actually keep a reference to the device,
    /// we just need to track its lifetime for correctness
    device: PhantomData<&'a Device>,
    // Technically this should be a map, as the ids can be re-used
    geometry: HashMap<u32, &'a Geometry>,
}
impl<'a> Scene<'a> {
    pub fn new(device: &'a Device) -> Scene {
        Scene {
            handle: unsafe { rtcNewScene(device.handle) },
            device: PhantomData,
            geometry: HashMap::new(),
        }
    }
    /// Attach a new geometry to the scene. Returns the scene local ID which
    /// can than be used to find the hit geometry from the ray ID member.
    pub fn attach_geometry(&mut self, mesh: &'a Geometry) -> u32 {
        let id = unsafe { rtcAttachGeometry(self.handle, mesh.handle()) };
        self.geometry.insert(id, mesh);
        id
    }
    /// Look up a geometry in the scene by the ID returned from `attach_geometry`
    pub fn get_geometry(&self, id: u32) -> Option<&'a Geometry> {
        match self.geometry.get(&id) {
            Some(g) => Some(*g),
            None => None,
        }
    }
    pub fn commit(&self) {
        unsafe {
            rtcCommitScene(self.handle);
        }
    }
    pub fn intersect(&self, ctx: &mut IntersectContext, ray: &mut RayHit) {
        unsafe {
            rtcIntersect1(
                self.handle,
                ctx as *mut RTCIntersectContext,
                ray as *mut RTCRayHit,
            );
        }
    }
    pub fn occluded(&self, ctx: &mut IntersectContext, ray: &mut Ray) {
        unsafe {
            rtcOccluded1(
                self.handle,
                ctx as *mut RTCIntersectContext,
                ray as *mut RTCRay,
            );
        }
    }
}

impl<'a> Drop for Scene<'a> {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseScene(self.handle);
        }
    }
}
