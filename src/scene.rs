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
    geometry: HashMap<u32, &'a Geometry>,
}

pub struct CommittedScene<'a> {
    scene: &'a Scene,
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
    // TODO: It makes sense to actually force the user to not
    // be able to modify the scene, after it's been committed. Since
    // at this point we consider the scene "built" b/c the BVH is built.
    // The "built" scene can then be used for intersect and occluded tests.
    // If the user wants to update the geometry in the scene they should
    // let the "built" scene go out of scope and be destroyed, then can
    // modify the geometry in the scene and re-commit it again to get
    // a "built" scene back for rendering again. This is sort of what I have
    // expressed (very badly) by right now having it require destroying
    // and re-creating the scene from scratch. I'm not sure how I can
    // do this built vs. non-built scene builder stuff without resorting to
    // runtime ownership checking though.
    // Maybe we can have this function return some object which takes ownership
    // of the map of geometry? It's a bit awkward because then as you attach
    // but haven't commited you can't modify anymore.
    pub fn commit(&'a self) -> CommittedScene<'a> {
        unsafe {
            rtcCommitScene(self.handle);
        }
        // TODO: This idea doesn't work, because the Scene then
        // loses the map of geometry IDs, and we can't give it
        // back without keeping the geometry locked out of being modified
        // because it remains borrowed by the scene again.
        // Runtime ownership may be the only option here unfortunately
        CommittedScene { scene: &self, geometry: self.geometry }
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

