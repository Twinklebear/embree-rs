use std::collections::HashMap;
use std::marker::PhantomData;
use std::mem;

use crate::device::Device;
use crate::geometry::Geometry;
use crate::ray::{IntersectContext, Ray, RayHit};
use crate::ray_packet::{Ray4, RayHit4};
use crate::ray_stream::{RayHitN, RayN};
use crate::sys::*;

/// A scene containing various geometry for rendering. Geometry
/// can be added and removed by attaching and detaching it, after
/// which the scene BVH can be built via `commit` which will
/// return a `CommittedScene` which can be used for ray queries.
pub struct Scene<'a> {
    pub(crate) handle: RTCScene,
    /// We don't need to actually keep a reference to the device,
    /// we just need to track its lifetime for correctness
    device: PhantomData<&'a Device>,
    geometry: HashMap<u32, Geometry<'a>>,
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
    /// Commit the scene to build the BVH on top of the geometry to allow
    /// for ray tracing the scene. The returned `CommittedScene` can be
    /// used for intersection and occlusion tests. The `Scene` can't
    /// be modified while the `CommittedScene` is active.
    pub fn commit(&'a self) -> CommittedScene<'a> {
        unsafe {
            rtcCommitScene(self.handle);
        }
        CommittedScene { scene: &self }
    }
    /// Get the underlying handle to the scene, e.g. for passing it to
    /// native code or ISPC kernels.
    pub unsafe fn handle(&self) -> RTCScene {
        self.handle
    }
}

impl<'a> Drop for Scene<'a> {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseScene(self.handle);
        }
    }
}

unsafe impl<'a> Sync for Scene<'a> {}

/// A committed scene with a BVH built over the geometry
/// which can be used for ray queries.
pub struct CommittedScene<'a> {
    pub(crate) scene: &'a Scene<'a>,
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
    pub fn intersect4(&self, ctx: &mut IntersectContext, ray: &mut RayHit4, valid: &[i32; 4]) {
        unsafe {
            rtcIntersect4(
                valid.as_ptr(),
                self.scene.handle,
                ctx as *mut RTCIntersectContext,
                ray as *mut RTCRayHit4,
            );
        }
    }
    pub fn occluded4(&self, ctx: &mut IntersectContext, ray: &mut Ray4, valid: &[i32; 4]) {
        unsafe {
            rtcOccluded4(
                valid.as_ptr(),
                self.scene.handle,
                ctx as *mut RTCIntersectContext,
                ray as *mut RTCRay4,
            );
        }
    }
    pub fn intersect_stream_aos(&self, ctx: &mut IntersectContext, rays: &mut Vec<RayHit>) {
        let m = rays.len();
        unsafe {
            rtcIntersect1M(
                self.scene.handle,
                ctx as *mut RTCIntersectContext,
                rays.as_mut_ptr(),
                m as u32,
                mem::size_of::<RayHit>(),
            );
        }
    }
    pub fn occluded_stream_aos(&self, ctx: &mut IntersectContext, rays: &mut Vec<Ray>) {
        let m = rays.len();
        unsafe {
            rtcOccluded1M(
                self.scene.handle,
                ctx as *mut RTCIntersectContext,
                rays.as_mut_ptr(),
                m as u32,
                mem::size_of::<Ray>(),
            );
        }
    }
    pub fn intersect_stream_soa(&self, ctx: &mut IntersectContext, rays: &mut RayHitN) {
        let n = rays.len();
        unsafe {
            let mut rayhit = rays.as_rayhitnp();
            rtcIntersectNp(
                self.scene.handle,
                ctx as *mut RTCIntersectContext,
                &mut rayhit as *mut RTCRayHitNp,
                n as u32,
            );
        }
    }
    pub fn occluded_stream_soa(&self, ctx: &mut IntersectContext, rays: &mut RayN) {
        let n = rays.len();
        unsafe {
            let mut r = rays.as_raynp();
            rtcOccludedNp(
                self.scene.handle,
                ctx as *mut RTCIntersectContext,
                &mut r as *mut RTCRayNp,
                n as u32,
            );
        }
    }
    pub fn bounds(&self) -> RTCBounds {
        let mut bounds = RTCBounds {
            lower_x: 0.0,
            upper_x: 0.0,
            lower_y: 0.0,
            upper_y: 0.0,
            lower_z: 0.0,
            upper_z: 0.0,
            align0: 0.0,
            align1: 0.0,
        };
        unsafe {
            rtcGetSceneBounds(self.handle(), &mut bounds as *mut RTCBounds);
        }
        bounds
    }
    /// Get the underlying handle to the scene, e.g. for passing it to
    /// native code or ISPC kernels.
    pub unsafe fn handle(&self) -> RTCScene {
        self.scene.handle
    }
}

unsafe impl<'a> Sync for CommittedScene<'a> {}
