use std::collections::HashMap;
use std::mem;
use std::sync::Arc;

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
pub struct Scene {
    pub(crate) handle: RTCScene,
    pub(crate) device: Arc<Device>,
    geometry: HashMap<u32, Arc<dyn Geometry>>,
}

impl Scene {
    pub fn new(device: Arc<Device>) -> Arc<Scene> {
        Arc::new(Scene {
            handle: unsafe { rtcNewScene(device.handle) },
            device: device,
            geometry: HashMap::new(),
        })
    }

    /// Attach a new geometry to the scene. Returns the scene local ID which
    /// can than be used to find the hit geometry from the ray ID member.
    /// A geometry can only be attached to one Scene at a time, per the Embree
    /// documentation. The geometry can be detached from the scene to move
    /// it to another one.
    pub fn attach_geometry(&mut self, mesh: Arc<dyn Geometry>) -> u32 {
        let id = unsafe { rtcAttachGeometry(self.handle, mesh.handle()) };
        self.geometry.insert(id, mesh);
        id
    }

    /// Detach the geometry from the scene
    pub fn deattach_geometry(&mut self, id: u32) {
        unsafe {
            rtcDetachGeometry(self.handle, id);
        }
        self.geometry.remove(&id);
    }

    /// Get the underlying handle to the scene, e.g. for passing it to
    /// native code or ISPC kernels.
    pub unsafe fn handle(&self) -> RTCScene {
        self.handle
    }

    /// Commit the scene to build the BVH on top of the geometry to allow
    /// for ray tracing the scene using the intersect/occluded methods
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

    pub fn intersect4(&self, ctx: &mut IntersectContext, ray: &mut RayHit4, valid: &[i32; 4]) {
        unsafe {
            rtcIntersect4(
                valid.as_ptr(),
                self.handle,
                ctx as *mut RTCIntersectContext,
                ray as *mut RTCRayHit4,
            );
        }
    }

    pub fn occluded4(&self, ctx: &mut IntersectContext, ray: &mut Ray4, valid: &[i32; 4]) {
        unsafe {
            rtcOccluded4(
                valid.as_ptr(),
                self.handle,
                ctx as *mut RTCIntersectContext,
                ray as *mut RTCRay4,
            );
        }
    }

    pub fn intersect_stream_aos(&self, ctx: &mut IntersectContext, rays: &mut Vec<RayHit>) {
        let m = rays.len();
        unsafe {
            rtcIntersect1M(
                self.handle,
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
                self.handle,
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
                self.handle,
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
                self.handle,
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
}

impl Drop for Scene {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseScene(self.handle);
        }
    }
}
