use crate::{Bounds, Error, SceneFlags};
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
    ray::{Ray, Ray4, RayHit, RayHit4, RayHitN, RayN},
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

    /// Returns the device the scene got created in.
    pub fn device(&self) -> &Device { &self.device }

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
    ///
    /// This function is NOT thread-safe, and thus CAN be used during rendering.
    /// However, it is recommended to store the geometry handle inside the
    /// application's geometry representation and look up the geometry
    /// handle from that representation directly.
    ///
    /// For a thread-safe version of this function, see [`Scene::get_geometry`].
    pub fn get_geometry_unchecked(&self, id: u32) -> Option<Geometry<'static>> {
        let raw = unsafe { rtcGetGeometry(self.handle, id) };
        if raw.is_null() {
            None
        } else {
            let geometries = self.geometries.lock().unwrap();
            geometries.get(&id).cloned()
        }
    }

    // TODO: add get_geometry with rtcGetGeometryThreadSafe

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

    /// Commits all changes for the specified scene.
    ///
    /// This internally triggers building of a spatial acceleration structure
    /// for the scene using all available worker threads. After the commit,
    /// ray queries can be executed on the scene.
    ///
    /// If scene geometries get modified or attached or detached, the
    /// [`Scene::commit`] call must be invoked before performing any further
    /// ray queries for the scene; otherwise the effect of the ray query is
    /// undefined.
    ///
    /// The modification of a geometry, committing the scene, and
    /// tracing of rays must always happen sequentially, and never at the
    /// same time.
    ///
    /// Any API call that sets a property of the scene or geometries
    /// contained in the scene count as scene modification, e.g. including
    /// setting of intersection filter functions.
    pub fn commit(&self) {
        unsafe {
            rtcCommitScene(self.handle);
        }
    }

    /// Commits the scene from multiple threads.
    ///
    /// This function is similar to [`Scene::commit`], but allows multiple
    /// threads to commit the scene at the same time. All threads must
    /// consistently call [`Scene::join_commit`].
    ///
    /// This method allows a flexible way to lazily create hierarchies
    /// during rendering. A thread reaching a not-yet-constructed sub-scene of a
    /// two-level scene can generate the sub-scene geometry and call this method
    /// on that just generated scene. During construction, further threads
    /// reaching the not-yet-built scene can join the build operation by
    /// also invoking this method. A thread that calls `join_commit` after
    /// the build finishes will directly return from the `join_commit` call.
    ///
    /// Multiple scene commit operations on different scenes can be running at
    /// the same time, hence it is possible to commit many small scenes in
    /// parallel, distributing the commits to many threads.
    pub fn join_commit(&self) {
        unsafe {
            rtcJoinCommitScene(self.handle);
        }
    }

    /// Set the scene flags. Multiple flags can be enabled using an OR
    /// operation. See [`RTCSceneFlags`] for all possible flags.
    /// On failure an error code is set that can be queried using
    /// [`rtcGetDeviceError`].
    ///
    /// Possible scene flags are:
    /// - NONE: No flags set.
    /// - DYNAMIC: Provides better build performance for dynamic scenes (but
    ///   also higher memory consumption).
    /// - COMPACT: Uses compact acceleration structures and avoids algorithms
    ///   that consume much memory.
    /// - ROBUST: Uses acceleration structures that allow for robust traversal,
    ///   and avoids optimizations that reduce arithmetic accuracy. This mode is
    ///   typically used for avoiding artifacts caused by rays shooting through
    ///   edges of neighboring primitives.
    /// - CONTEXT_FILTER_FUNCTION: Enables support for a filter function inside
    ///   the intersection context for this scene.
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
    /// let flags = scene.get_flags();
    /// scene.set_flags(flags | SceneFlags::ROBUST);
    /// ```
    pub fn get_flags(&self) -> RTCSceneFlags { unsafe { rtcGetSceneFlags(self.handle) } }

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
    /// Analogous to [`sys::rtcIntersect1`].
    ///
    /// The user has to initialize the ray origin, ray direction, ray segment
    /// (`tnear`, `tfar` ray members), and set the ray flags to 0 (`flags` ray
    /// member). If the scene contains motion blur geometries, also the ray
    /// time (`time` ray member) must be initialized to a value in the range
    /// [0, 1]. If ray masks are enabled at compile time, the ray mask
    /// (`mask` ray member) must be initialized as well. The geometry ID
    /// (`geomID` ray hit member) must be initialized to `INVALID_ID`.
    ///
    /// When no intersection is found, the ray/hit data is not updated. When an
    /// intersection is found, the hit distance is written into the `tfar`
    /// member of the ray and all hit data is set, such as unnormalized
    /// geometry normal in object space (`Ng` hit member), local hit
    /// coordinates (`u, v` hit member), instance ID stack (`instID`
    /// hit member), geometry ID (`geomID` hit member), and primitive ID
    /// (`primID` hit member). See [`RayHit`] for more information.
    ///
    /// The intersection context (`ctx` argument) can specify flags to optimize
    /// traversal and a filter callback function to be invoked for every
    /// intersection. Further, the pointer to the intersection context is
    /// propagated to callback functions invoked during traversal and can
    /// thus be used to extend the ray with additional data. See
    /// [`IntersectContext`] for more information.
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
                <&mut Ray as Into<&mut RTCRay>>::into(ray) as *mut RTCRay,
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
                rays.as_mut_ptr() as *mut RTCRay,
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

    /// Returns the axis-aligned bounding box of the scene.
    pub fn get_bounds(&self) -> Bounds {
        let mut bounds = Bounds {
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
            rtcGetSceneBounds(self.handle(), &mut bounds as *mut Bounds);
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
