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
    geometry: HashMap<u32, Geometry<'a>>,
}

pub struct CommittedScene<'a> {
    pub (crate) scene: &'a Scene<'a>,
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
    /// A geometry can only be attached to one Scene at a time, per the Embree
    /// documentation. The geometry can be detached from the scene to move
    /// it to another one.
    pub fn attach_geometry(&mut self, mesh: Geometry<'a>) -> u32 {
        let id = unsafe { rtcAttachGeometry(self.handle, mesh.handle()) };
        self.geometry.insert(id, mesh);
        id
    }
    /// Detach the geometry from the scene
    pub fn deattach_geometry(&mut self, id: u32) -> Option<Geometry<'a>> {
        self.geometry.remove(&id)
    }
    /// Look up a geometry in the scene by the ID returned from `attach_geometry`
    pub fn get_geometry(&self, id: u32) -> Option<&Geometry<'a>> {
        match self.geometry.get(&id) {
            Some(g) => Some(g),
            None => None,
        }
    }
    /// Look up a geometry in the scene by the ID returned from `attach_geometry`
    pub fn get_geometry_mut(&mut self, id: u32) -> Option<&mut Geometry<'a>> {
        match self.geometry.get_mut(&id) {
            Some(g) => Some(g),
            None => None,
        }
    }
    /// Get an iterator over the geometry map
    pub fn iter(&self) -> std::collections::hash_map::Iter<u32, Geometry<'a>> {
        self.geometry.iter()
    }
    /// Get an iterator over the geometry map
    pub fn iter_mut(&mut self) -> std::collections::hash_map::IterMut<u32, Geometry<'a>> {
        self.geometry.iter_mut()
    }
    /// TODO DOC
    pub fn commit(&'a self) -> CommittedScene<'a> {
        unsafe {
            rtcCommitScene(self.handle);
        }
        CommittedScene { scene: &self }
    }
}

impl<'a> Drop for Scene<'a> {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseScene(self.handle);
        }
    }
}

impl<'a> CommittedScene<'a> {
    pub fn intersect(&self, ctx: &mut IntersectContext, ray: &mut RayHit) {
        unsafe {
            rtcIntersect1(
                self.scene.handle,
                ctx as *mut RTCIntersectContext,
                ray as *mut RTCRayHit,
            );
        }
    }
    pub fn occluded(&self, ctx: &mut IntersectContext, ray: &mut Ray) {
        unsafe {
            rtcOccluded1(
                self.scene.handle,
                ctx as *mut RTCIntersectContext,
                ray as *mut RTCRay,
            );
        }
    }
}

