use crate::{
    Bounds, Error, PointQuery, PointQueryContext, Ray16, Ray8, RayHit16, RayHit8, RayHitNp,
    RayHitPacket, RayPacket, SceneFlags,
};
use std::{
    any::TypeId,
    collections::HashMap,
    mem, ptr,
    sync::{Arc, Mutex},
};

use crate::{
    context::IntersectContext,
    device::Device,
    geometry::Geometry,
    ray::{Ray, Ray4, RayHit, RayHit4, RayNp},
    sys::*,
};

/// A scene containing various geometries.
#[derive(Debug)]
pub struct Scene<'a> {
    pub(crate) handle: RTCScene,
    pub(crate) device: Device,
    geometries: Arc<Mutex<HashMap<u32, Geometry<'a>>>>,
    point_query_user_data: Arc<Mutex<PointQueryUserData>>,
}

impl<'a> Clone for Scene<'a> {
    fn clone(&self) -> Self {
        unsafe { rtcRetainScene(self.handle) }
        Self {
            handle: self.handle,
            device: self.device.clone(),
            geometries: self.geometries.clone(),
            point_query_user_data: self.point_query_user_data.clone(),
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

unsafe impl<'a> Sync for Scene<'a> {}
unsafe impl<'a> Send for Scene<'a> {}

impl<'a> Scene<'a> {
    /// Creates a new scene with the given device.
    pub(crate) fn new(device: Device) -> Result<Self, Error> {
        let handle = unsafe { rtcNewScene(device.handle) };
        if handle.is_null() {
            Err(device.get_error())
        } else {
            Ok(Scene {
                handle,
                device,
                geometries: Default::default(),
                point_query_user_data: Arc::new(Mutex::new(PointQueryUserData::default())),
            })
        }
    }

    /// Creates a new scene with the given device and flags.
    pub(crate) fn new_with_flags(device: Device, flags: SceneFlags) -> Result<Self, Error> {
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
    pub fn attach_geometry(&mut self, geometry: &Geometry<'a>) -> u32 {
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
    pub fn attach_geometry_by_id(&mut self, geometry: &Geometry<'a>, id: u32) {
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
    pub fn get_geometry_unchecked(&self, id: u32) -> Option<Geometry<'a>> {
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

    /// Traverses the BVH with a point query object.
    ///
    /// Traverses the BVH using the point query object and calls a user defined
    /// callback function for each primitive of the scene that intersects the
    /// query domain.
    ///
    /// The user has to initialize the query location (x, y and z member) and
    /// query radius in the range [0, ∞]. If the scene contains motion blur
    /// geometries, also the query time (time member) must be initialized to
    /// a value in the range [0, 1].
    ///
    /// # Arguments
    ///
    /// * `query` - The point query object.
    ///
    /// * `context` - The point query context object. It contains ID and
    ///   transformation information of the instancing hierarchy if
    ///   (multilevel-)instancing is used. See [`PointQueryContext`].
    ///
    /// * `query_fn` - The user defined callback function. For each primitive
    ///   that intersects the query domain, the callback function is called, in
    ///   which distance computations to the primitive can be implemented. The
    ///   user will be provided with the primitive ID and geometry ID of the
    ///   according primitive, however, the geometry information has to be
    ///   determined manually. The callback function can be `None`, in which
    ///   case the callback function is not invoked.
    ///
    /// * `user_data` - The user defined data that is passed to the callback.
    ///
    /// A callback function can still get attached to a specific [`Geometry`]
    /// object using [`Geometry::set_point_query_function`]. If a callback
    /// function is attached to a geometry, and (a potentially different)
    /// callback function is passed to this function, both functions will be
    /// called for the primitives of the according geometries.
    ///
    /// The query radius can be decreased inside the callback function, which
    /// allows to efficiently cull parts of the scene during BVH traversal.
    /// Increasing the query radius and modifying time or location of the query
    /// will result in undefined behavior.
    ///
    /// The callback function will be called for all primitives in a leaf node
    /// of the BVH even if the primitive is outside the query domain,
    /// since Embree does not gather geometry information of primitives
    /// internally.
    ///
    /// Point queries can be used with (multi-)instancing. However, care has to
    /// be taken when the instance transformation contains anisotropic scaling
    /// or sheering. In these cases distance computations have to be performed
    /// in world space to ensure correctness and the ellipsoidal query domain
    /// (in instance space) will be approximated with its axis aligned
    /// bounding box internally. Therefore, the callback function might be
    /// invoked even for primitives in inner BVH nodes that do not intersect
    /// the query domain.
    ///
    /// The point query structure must be aligned to 16 bytes.
    ///
    /// Currently, all primitive types are supported by the point query API
    /// except of points (see [`GeometryKind::POINT`]), curves (see
    /// [`GeometryKind::CURVE`]) and subdivision surfaces (see
    /// [`GeometryKind::SUBDIVISION]).
    ///
    /// See **closet_point** in examples folder for an example of this.
    pub fn point_query<F, D>(
        &self,
        query: &mut PointQuery,
        context: &mut PointQueryContext,
        query_fn: Option<F>,
        mut user_data: Option<D>,
    ) where
        D: UserPointQueryData,
        F: FnMut(&mut PointQuery, &mut PointQueryContext, Option<&mut D>, u32, u32, f32) -> bool,
    {
        let mut query_fn = query_fn;
        let point_query_user_data = PointQueryUserData {
            scene_closure: if query_fn.is_some() {
                query_fn.as_mut().unwrap() as *mut F as *mut _
            } else {
                ptr::null_mut()
            },
            data: if user_data.is_some() {
                user_data.as_mut().unwrap() as *mut D as *mut _
            } else {
                ptr::null_mut()
            },
            type_id: TypeId::of::<D>(),
        };
        unsafe {
            rtcPointQuery(
                self.handle,
                query as *mut _,
                context as *mut _,
                if query_fn.is_some() {
                    point_query_function(query_fn.as_mut().unwrap())
                } else {
                    None
                },
                if query_fn.is_some() {
                    point_query_user_data.data as *mut D as *mut _
                } else {
                    std::ptr::null_mut()
                },
            );
        }
    }

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
    pub fn set_progress_monitor_function<F>(&mut self, progress: F)
    where
        F: FnMut(f64) -> bool,
    {
        unsafe {
            let mut closure = progress;
            rtcSetSceneProgressMonitorFunction(
                self.handle,
                progress_monitor_function(&mut closure),
                &mut closure as *mut _ as *mut ::std::os::raw::c_void,
            );
        }
    }

    /// Unregister the progress monitor callback function.
    pub fn unset_progress_monitor_function(&mut self) {
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
    /// * `ctx` - The intersection context to use for the ray query. It
    ///   specifies flags to optimize traversal and a filter callback function
    ///   to be invoked for every intersection. Further, the pointer to the
    ///   intersection context is propagated to callback functions invoked
    ///   during traversal and can thus be used to extend the ray with
    ///   additional data. See [`IntersectContext`] for more information.
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

    /// Finds the closest hits for a ray packet of size 4 with the scene.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The intersection context to use for the ray query. It
    ///   specifies flags to optimize traversal and a filter callback function
    ///   to be invoked for every intersection. Further, the pointer to the
    ///   intersection context is propagated to callback functions invoked
    ///   during traversal and can thus be used to extend the ray with
    ///   additional data. See [`IntersectContext`] for more information.
    /// * `ray` - The ray packet of size 4 to intersect with the scene. The ray
    ///   packet must be aligned to 16 bytes.
    /// * `valid` - A mask indicating which rays in the packet are valid. -1
    ///   means
    ///  valid, 0 means invalid.
    ///
    /// The ray packet pointer passed to callback functions is not guaranteed to
    /// be identical to the original ray provided. To extend the ray with
    /// additional data to be accessed in callback functions, use the
    /// intersection context.
    ///
    /// Only active rays are processed, and hit data of inactive rays is not
    /// changed.
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

    /// Finds the closest hits for a ray packet of size 8 with the scene.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The intersection context to use for the ray query. It
    ///   specifies flags to optimize traversal and a filter callback function
    ///   to be invoked for every intersection. Further, the pointer to the
    ///   intersection context is propagated to callback functions invoked
    ///   during traversal and can thus be used to extend the ray with
    ///   additional data. See [`IntersectContext`] for more information.
    /// * `ray` - The ray packet of size 8 to intersect with the scene. The ray
    ///   packet must be aligned to 32 bytes.
    /// * `valid` - A mask indicating which rays in the packet are valid. -1
    ///   means
    ///  valid, 0 means invalid.
    ///
    /// The ray packet pointer passed to callback functions is not guaranteed to
    /// be identical to the original ray provided. To extend the ray with
    /// additional data to be accessed in callback functions, use the
    /// intersection context.
    ///
    /// Only active rays are processed, and hit data of inactive rays is not
    /// changed.
    pub fn intersect8(&self, ctx: &mut IntersectContext, ray: &mut RayHit8, valid: &[i32; 8]) {
        unsafe {
            rtcIntersect8(
                valid.as_ptr(),
                self.handle,
                ctx as *mut RTCIntersectContext,
                ray as *mut RTCRayHit8,
            );
        }
    }

    /// Finds the closest hits for a ray packet of size 16 with the scene.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The intersection context to use for the ray query. It
    ///   specifies flags to optimize traversal and a filter callback function
    ///   to be invoked for every intersection. Further, the pointer to the
    ///   intersection context is propagated to callback functions invoked
    ///   during traversal and can thus be used to extend the ray with
    ///   additional data. See [`IntersectContext`] for more information.
    /// * `ray` - The ray packet of size 16 to intersect with the scene. The ray
    ///   packet must be aligned to 64 bytes.
    /// * `valid` - A mask indicating which rays in the packet are valid. -1
    ///   means
    ///  valid, 0 means invalid.
    ///
    /// The ray packet pointer passed to callback functions is not guaranteed to
    /// be identical to the original ray provided. To extend the ray with
    /// additional data to be accessed in callback functions, use the
    /// intersection context.
    ///
    /// Only active rays are processed, and hit data of inactive rays is not
    /// changed.
    pub fn intersect16(&self, ctx: &mut IntersectContext, ray: &mut RayHit16, valid: &[i32; 16]) {
        unsafe {
            rtcIntersect16(
                valid.as_ptr(),
                self.handle,
                ctx as *mut RTCIntersectContext,
                ray as *mut RTCRayHit16,
            );
        }
    }

    /// Checks for a single ray if whether there is any hit with the scene.
    ///
    /// When no intersection is found, the ray data is not updated. In case
    /// a hit was found, the `tfar` component of the ray is set to `-inf`.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The intersection context to use for the ray query. It
    ///   specifies flags to optimize traversal and a filter callback function
    ///   to be invoked for every intersection. Further, the pointer to the
    ///   intersection context is propagated to callback functions invoked
    ///   during traversal and can thus be used to extend the ray with
    ///   additional data. See [`IntersectContext`] for more information.
    ///
    /// * `ray` - The ray to intersect with the scene.
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

    /// Checks for each active ray of a ray packet of size 4 if whether there is
    /// any hit with the scene.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The intersection context to use for the ray query. It
    ///   specifies flags to optimize traversal and a filter callback function
    ///   to be invoked for every intersection. Further, the pointer to the
    ///   intersection context is propagated to callback functions invoked
    ///   during traversal and can thus be used to extend the ray with
    ///   additional data. See [`IntersectContext`] for more information.
    /// * `ray` - The ray packet of size 4 to intersect with the scene. The ray
    ///   packet must be aligned to 16 bytes.
    /// * `valid` - A mask indicating which rays in the packet are valid. -1
    ///   means
    ///  valid, 0 means invalid.
    ///
    /// The ray packet pointer passed to callback functions is not guaranteed to
    /// be identical to the original ray provided. To extend the ray with
    /// additional data to be accessed in callback functions, use the
    /// intersection context.
    ///
    /// Only active rays are processed, and hit data of inactive rays is not
    /// changed.
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

    /// Checks for each active ray of a ray packet of size 4 if whether there is
    /// any hit with the scene.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The intersection context to use for the ray query. It
    ///   specifies flags to optimize traversal and a filter callback function
    ///   to be invoked for every intersection. Further, the pointer to the
    ///   intersection context is propagated to callback functions invoked
    ///   during traversal and can thus be used to extend the ray with
    ///   additional data. See [`IntersectContext`] for more information.
    /// * `ray` - The ray packet of size 8 to intersect with the scene. The ray
    ///   packet must be aligned to 32 bytes.
    /// * `valid` - A mask indicating which rays in the packet are valid. -1
    ///   means
    ///  valid, 0 means invalid.
    ///
    /// The ray packet pointer passed to callback functions is not guaranteed to
    /// be identical to the original ray provided. To extend the ray with
    /// additional data to be accessed in callback functions, use the
    /// intersection context.
    ///
    /// Only active rays are processed, and hit data of inactive rays is not
    /// changed.
    pub fn occluded8(&self, ctx: &mut IntersectContext, ray: &mut Ray8, valid: &[i32; 8]) {
        unsafe {
            rtcOccluded8(
                valid.as_ptr(),
                self.handle,
                ctx as *mut RTCIntersectContext,
                ray as *mut RTCRay8,
            );
        }
    }

    /// Checks for each active ray of a ray packet of size 16 if whether there
    /// is any hit with the scene.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The intersection context to use for the ray query. It
    ///   specifies flags to optimize traversal and a filter callback function
    ///   to be invoked for every intersection. Further, the pointer to the
    ///   intersection context is propagated to callback functions invoked
    ///   during traversal and can thus be used to extend the ray with
    ///   additional data. See [`IntersectContext`] for more information.
    /// * `ray` - The ray packet of size 16 to intersect with the scene. The ray
    ///   packet must be aligned to 64 bytes.
    /// * `valid` - A mask indicating which rays in the packet are valid. -1
    ///   means
    ///  valid, 0 means invalid.
    ///
    /// The ray packet pointer passed to callback functions is not guaranteed to
    /// be identical to the original ray provided. To extend the ray with
    /// additional data to be accessed in callback functions, use the
    /// intersection context.
    ///
    /// Only active rays are processed, and hit data of inactive rays is not
    /// changed.
    pub fn occluded16(&self, ctx: &mut IntersectContext, ray: &mut Ray16, valid: &[i32; 16]) {
        unsafe {
            rtcOccluded16(
                valid.as_ptr(),
                self.handle,
                ctx as *mut RTCIntersectContext,
                ray as *mut RTCRay16,
            );
        }
    }

    /// Finds the closest hits for a stream of M ray packets.
    ///
    /// A ray in the stream is inactive if its `tnear` value is larger than its
    /// `tfar` value. The stream can be any size including zero. Each ray
    /// must be aligned to 16 bytes.
    ///
    /// The implementation of the stream ray query functions may re-order rays
    /// arbitrarily and re-pack rays into ray packets of different size. For
    /// this reason, the callback functions may be invoked with an arbitrary
    /// packet size (of size 1, 4, 8, or 16) and different ordering as
    /// specified initially in the ray stream. For this reason, you MUST NOT
    /// rely on the ordering of the rays in the ray stream to be preserved but
    /// instead use the `rayID` component of the ray to identify the original
    /// rya, e.g. to access a per-ray payload.
    ///
    /// Analogous to [`rtcIntersectNM`] and [`rtcIntersect1M`].
    ///
    /// # Arguments
    ///
    /// * `ctx` - The intersection context to use for the ray query. It
    ///   specifies flags to optimize traversal and a filter callback function
    ///   to be invoked for every intersection. Further, the pointer to the
    ///   intersection context is propagated to callback functions invoked
    ///   during traversal and can thus be used to extend the ray with
    ///   additional data. See [`IntersectContext`] for more information.
    ///
    /// * `rays` - The ray stream to intersect with the scene.
    pub fn intersect_stream_aos<P: RayHitPacket>(
        &self,
        ctx: &mut IntersectContext,
        rays: &mut Vec<P>,
    ) {
        let m = rays.len();
        unsafe {
            if P::Ray::LEN == 1 {
                rtcIntersect1M(
                    self.handle,
                    ctx as *mut RTCIntersectContext,
                    rays.as_mut_ptr() as *mut _,
                    m as u32,
                    mem::size_of::<P>(),
                );
            } else {
                rtcIntersectNM(
                    self.handle,
                    ctx as *mut RTCIntersectContext,
                    rays.as_mut_ptr() as *mut _,
                    P::Ray::LEN as u32,
                    m as u32,
                    mem::size_of::<P>(),
                );
            }
        }
    }

    /// Finds the closest hits for a stream of M ray packets.
    ///
    /// A ray in the stream is inactive if its `tnear` value is larger than its
    /// `tfar` value. The stream can be any size including zero. Each ray
    /// must be aligned to 16 bytes.
    ///
    /// The implementation of the stream ray query functions may re-order rays
    /// arbitrarily and re-pack rays into ray packets of different size. For
    /// this reason, the callback functions may be invoked with an arbitrary
    /// packet size (of size 1, 4, 8, or 16) and different ordering as
    /// specified initially in the ray stream. For this reason, you MUST NOT
    /// rely on the ordering of the rays in the ray stream to be preserved but
    /// instead use the `rayID` component of the ray to identify the original
    /// rya, e.g. to access a per-ray payload.
    ///
    /// Analogous to [`rtcOccluded1M`] and [`rtcOccludedNM`].
    ///
    /// # Arguments
    ///
    /// * `ctx` - The intersection context to use for the ray query. It
    ///   specifies flags
    /// to optimize traversal and a filter callback function to be invoked for
    /// every intersection. Further, the pointer to the intersection context
    /// is propagated to callback functions invoked during traversal and can
    /// thus be used to extend the ray with additional data. See
    /// [`IntersectContext`] for more information.
    ///
    /// * `rays` - The ray stream to intersect with the scene.
    pub fn occluded_stream_aos<P: RayPacket>(&self, ctx: &mut IntersectContext, rays: &mut Vec<P>) {
        let m = rays.len();
        unsafe {
            if P::LEN == 1 {
                rtcOccluded1M(
                    self.handle,
                    ctx as *mut RTCIntersectContext,
                    rays.as_mut_ptr() as *mut RTCRay,
                    m as u32,
                    mem::size_of::<P>(),
                );
            } else {
                rtcOccludedNM(
                    self.handle,
                    ctx as *mut RTCIntersectContext,
                    rays.as_mut_ptr() as *mut RTCRayN,
                    P::LEN as u32,
                    m as u32,
                    mem::size_of::<P>(),
                );
            }
        }
    }

    /// Finds the closest hit for a SOA ray stream of size `n`.
    ///
    /// The implementation of the stream ray query functions may re-order rays
    /// arbitrarily and re-pack rays into ray packets of different size. For
    /// this reason, callback functions may be invoked with an arbitrary
    /// packet size (of size 1, 4, 8, or 16) and different ordering as
    /// specified initially. For this reason, one may have to use the rayID
    /// component of the ray to identify the original ray, e.g. to access
    /// a per-ray payload.
    ///
    /// A ray in a ray stream is considered inactive if its tnear value is
    /// larger than its tfar value.
    pub fn intersect_stream_soa(&self, ctx: &mut IntersectContext, rays: &mut RayHitNp) {
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

    /// Finds any hits for a SOA ray stream of size `n`.
    ///
    /// The implementation of the stream ray query functions may re-order rays
    /// arbitrarily and re-pack rays into ray packets of different size. For
    /// this reason, callback functions may be invoked with an arbitrary
    /// packet size (of size 1, 4, 8, or 16) and different ordering as
    /// specified initially. For this reason, one may have to use the rayID
    /// component of the ray to identify the original ray, e.g. to access
    /// a per-ray payload.
    ///
    /// A ray in a ray stream is considered inactive if its tnear value is
    /// larger than its tfar value.
    pub fn occluded_stream_soa(&self, ctx: &mut IntersectContext, rays: &mut RayNp) {
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

pub trait UserPointQueryData: Sized + Send + Sync + 'static {}

impl<T> UserPointQueryData for T where T: Sized + Send + Sync + 'static {}

/// User data for callback of [`Scene::point_query`] and
/// [`Geometry::set_point_query_function`].
#[derive(Debug)]
pub(crate) struct PointQueryUserData {
    pub scene_closure: *mut std::os::raw::c_void,
    pub data: *mut std::os::raw::c_void,
    pub type_id: TypeId,
}

impl Default for PointQueryUserData {
    fn default() -> Self {
        Self {
            scene_closure: ptr::null_mut(),
            data: ptr::null_mut(),
            type_id: TypeId::of::<()>(),
        }
    }
}

/// Helper function to convert a Rust closure to `RTCProgressMonitorFunction`
/// callback.
fn progress_monitor_function<F>(_f: &mut F) -> RTCProgressMonitorFunction
where
    F: FnMut(f64) -> bool,
{
    unsafe extern "C" fn inner<F>(f: *mut std::os::raw::c_void, n: f64) -> bool
    where
        F: FnMut(f64) -> bool,
    {
        let cb = &mut *(f as *mut F);
        cb(n)
    }

    Some(inner::<F>)
}

/// Helper function to convert a Rust closure to `RTCPointQueryFunction`
/// callback.
fn point_query_function<F, D>(_f: &mut F) -> RTCPointQueryFunction
where
    D: UserPointQueryData,
    F: FnMut(&mut PointQuery, &mut PointQueryContext, Option<&mut D>, u32, u32, f32) -> bool,
{
    unsafe extern "C" fn inner<F, D>(args: *mut RTCPointQueryFunctionArguments) -> bool
    where
        D: UserPointQueryData,
        F: FnMut(&mut PointQuery, &mut PointQueryContext, Option<&mut D>, u32, u32, f32) -> bool,
    {
        let user_data = &mut *((*args).userPtr as *mut PointQueryUserData);
        let cb_ptr = user_data.scene_closure as *mut F;
        if !cb_ptr.is_null() {
            let data = {
                if user_data.data.is_null() || user_data.type_id != TypeId::of::<D>() {
                    None
                } else {
                    Some(&mut *(user_data.data as *mut D))
                }
            };
            let cb = &mut *cb_ptr;
            cb(
                &mut *(*args).query,
                &mut *(*args).context,
                data,
                (*args).primID,
                (*args).geomID,
                (*args).similarityScale,
            )
        } else {
            false
        }
    }

    Some(inner::<F, D>)
}

// TODO: implement rtcIntersect1Mp
