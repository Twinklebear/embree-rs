use crate::{Error, SceneFlags};
use std::{
    collections::HashMap,
    mem,
    sync::{Arc, Mutex},
};

use crate::{
    callback,
    device::Device,
    geometry::Geometry,
    intersect_context::IntersectContext,
    ray::{Ray, RayHit},
    ray_packet::{Ray4, RayHit4},
    ray_stream::{RayHitN, RayN},
    sys::*,
};

/// A scene containing various geometries.
#[derive(Debug)]
pub struct Scene {
    pub(crate) handle: RTCScene,
    pub(crate) device: Device,
    geometries: Arc<Mutex<HashMap<u32, Geometry<'static>>>>,
}

impl Clone for Scene {
    fn clone(&self) -> Self {
        unsafe { rtcRetainScene(self.handle) }
        Self {
            handle: self.handle,
            device: self.device.clone(),
            geometries: self.geometries.clone(),
        }
    }
}

impl Scene {
    /// Creates a new scene with the given device.
    pub(crate) fn new(device: Device) -> Result<Scene, Error> {
        let handle = unsafe { rtcNewScene(device.handle) };
        if handle.is_null() {
            Err(device.get_error())
        } else {
            Ok(Scene {
                handle,
                device,
                geometries: Default::default(),
            })
        }
    }

    /// Creates a new scene with the given device and flags.
    pub(crate) fn new_with_flags(device: Device, flags: SceneFlags) -> Result<Scene, Error> {
        let scene = Self::new(device)?;
        scene.set_flags(flags);
        Ok(scene)
    }

    /// Attaches a new geometry to the scene.
    ///
    /// A geometry can get attached to multiple scenes. The geometry ID is
    /// unique per scene, and is used to identify the geometry when hitting
    /// by a ray or ray packet during ray queries.
    ///
    /// This function is thread-safe, thus multiple threads can attach
    /// geometries to a scene at the same time.
    ///
    /// The geometry IDs are assigned sequentially, starting at 0, as long as
    /// no geometries are detached from the scene. If geometries are detached
    /// from the scene, the implementation will reuse IDs in an implementation
    /// dependent way.
    pub fn attach_geometry<'a>(&'a mut self, geometry: &'a Geometry<'static>) -> u32 {
        let id = unsafe { rtcAttachGeometry(self.handle, geometry.handle) };
        self.geometries.lock().unwrap().insert(id, geometry.clone());
        id
    }

    /// Attaches a geometry to the scene using a specified geometry ID.
    ///
    /// A geometry can get attached to multiple scenes. The user-provided
    /// geometry ID must be unused in the scene, otherwise the creation of the
    /// geometry will fail. Further, the user-provided geometry IDs
    /// should be compact, as Embree internally creates a vector which size is
    /// equal to the largest geometry ID used. Creating very large geometry
    /// IDs for small scenes would thus cause a memory consumption and
    /// performance overhead.
    ///
    /// This function is thread-safe, thus multiple threads can attach
    /// geometries to a scene at the same time.
    pub fn attach_geometry_by_id<'a>(&'a mut self, geometry: &'a Geometry<'static>, id: u32) {
        unsafe { rtcAttachGeometryByID(self.handle, geometry.handle, id) };
        self.geometries.lock().unwrap().insert(id, geometry.clone());
    }

    /// Detaches the geometry from the scene.
    ///
    /// This function is thread-safe, thus multiple threads can detach
    /// geometries from a scene at the same time.
    pub fn detach_geometry(&mut self, id: u32) {
        unsafe {
            rtcDetachGeometry(self.handle, id);
        }
        self.geometries.lock().unwrap().remove(&id);
    }

    /// Returns the geometry bound to the specified geometry ID.
    pub fn get_geometry(&self, id: u32) -> Option<Geometry> {
        let geometry = unsafe { rtcGetGeometry(self.handle, id) };
        if geometry.is_null() {
            None
        } else {
            Some(Geometry::new(self.device))
        }
    }

    /// Returns the raw underlying handle to the scene, e.g. for passing it to
    /// native code or ISPC kernels.
    ///
    /// # Safety
    ///
    /// Use this function only if you know what you are doing. The returned
    /// handle is a raw pointer to an Embree reference-counted object. The
    /// reference count is not increased by this function, so the caller must
    /// ensure that the handle is not used after the scene object is
    /// destroyed.
    pub unsafe fn handle(&self) -> RTCScene { self.handle }

    /// Commit the scene to build the BVH on top of the geometry to allow
    /// for ray tracing the scene using the intersect/occluded methods
    pub fn commit(&self) {
        unsafe {
            rtcCommitScene(self.handle);
        }
    }

    /// Set the scene flags. Multiple flags can be enabled using an OR
    /// operation. See [`RTCSceneFlags`] for all possible flags.
    /// On failure an error code is set that can be queried using
    /// [`rtcGetDeviceError`].
    pub fn set_flags(&self, flags: RTCSceneFlags) {
        unsafe {
            rtcSetSceneFlags(self.handle, flags);
        }
    }

    /// Query the flags of the scene.
    ///
    /// Useful when setting individual flags, e.g. to just set the robust mode
    /// without changing other flags the following way:
    /// ```no_run
    /// use embree::{Device, Scene, SceneFlags};
    /// let device = Device::new().unwrap();
    /// let scene = device.create_scene().unwrap();
    /// let flags = scene.flags();
    /// scene.set_flags(flags | SceneFlags::ROBUST);
    /// ```
    pub fn flags(&self) -> RTCSceneFlags { unsafe { rtcGetSceneFlags(self.handle) } }

    /// Set the build quality of the scene. See [`RTCBuildQuality`] for all
    /// possible values.
    ///
    /// The per-geometry build quality is only a hint and may be ignored. Embree
    /// currently uses the per-geometry build quality when the scene build
    /// quality is set to [`BuildQuality::LOW`]. In this mode a two-level
    /// acceleration structure is build, and geometries build a separate
    /// acceleration structure using the geometry build quality.
    ///
    /// The build quality can be one of the following:
    ///
    /// - [`BuildQuality::LOW`]: Creates lower quality data structures, e.g. for
    ///   dynamic scenes.
    ///
    /// - [`BuildQuality::MEDIUM`]: Default build quality for most usages. Gives
    ///   a good balance between quality and performance.
    ///
    /// - [`BuildQuality::HIGH`]: Creates higher quality data structures for
    ///   final frame rendering. Enables a spatial split builder for certain
    ///   primitive types.
    ///
    /// - [`BuildQuality::REFIT`]: Uses a BVH refitting approach when changing
    ///   only the vertex buffer.
    pub fn set_build_quality(&self, quality: RTCBuildQuality) {
        unsafe {
            rtcSetSceneBuildQuality(self.handle, quality);
        }
    }

    /// Register a progress monitor callback function.
    ///
    /// Only one progress monitor callback can be registered per scene,
    /// and further invocations overwrite the previously registered callback.
    ///
    /// Unregister with [`Scene::unset_progress_monitor_function`].
    ///
    /// # Arguments
    ///
    /// * `progress` - A callback function that takes a number in range [0.0,
    ///   1.0]
    /// indicating the progress of the operation.
    ///
    /// # Warning
    ///
    /// Must be called after the scene has been committed.
    pub fn set_progress_monitor_function<F>(&self, progress: F)
    where
        F: FnMut(f64) -> bool,
    {
        unsafe {
            let mut closure = progress;

            rtcSetSceneProgressMonitorFunction(
                self.handle,
                callback::progress_monitor_function_helper(&mut closure),
                &mut closure as *mut _ as *mut ::std::os::raw::c_void,
            );
        }
    }

    /// Unregister the progress monitor callback function.
    pub fn unset_progress_monitor_function(&self) {
        unsafe {
            rtcSetSceneProgressMonitorFunction(self.handle, None, ::std::ptr::null_mut());
        }
    }

    /// Finds the closest hit of a single ray with the scene.
    ///
    /// Analogous to [`rtcIntersect1`].
    ///
    /// # Arguments
    ///
    /// * `ctx` - The intersection context to use for the ray query.
    /// * `ray` - The ray to intersect with the scene.
    pub fn intersect(&self, ctx: &mut IntersectContext, ray: Ray) -> RayHit {
        let mut ray_hit = RayHit::new(ray);
        unsafe {
            rtcIntersect1(
                self.handle,
                ctx as *mut RTCIntersectContext,
                &mut ray_hit as *mut RTCRayHit,
            );
        }
        ray_hit
    }

    /// Checks for a single ray if whether there is any hit with the scene.
    pub fn occluded(&self, ctx: &mut IntersectContext, ray: &mut Ray) -> bool {
        unsafe {
            rtcOccluded1(
                self.handle,
                ctx as *mut RTCIntersectContext,
                ray as *mut RTCRay,
            );
        }
        ray.tfar == -f32::INFINITY
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
