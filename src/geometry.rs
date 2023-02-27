use std::{
    any::TypeId, collections::HashMap, marker::PhantomData, num::NonZeroUsize, ptr, sync::Mutex,
};

use crate::{
    sys::*, Bounds, BufferSlice, BufferUsage, BuildQuality, Device, Error, Format, GeometryKind,
    HitN, IntersectContext, QuaternionDecomposition, RayHitN, RayN, Scene, SubdivisionMode,
};

use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
    sync::Arc,
};

// TODO(yang): maybe enforce format and stride when get the view?
/// Information about how a (part of) buffer is bound to a geometry.
#[derive(Debug, Clone)]
pub(crate) struct AttachedBuffer<'src> {
    slot: u32,
    #[allow(dead_code)]
    format: Format,
    #[allow(dead_code)]
    stride: usize,
    source: BufferSlice<'src>,
}

/// Trait for user-defined data that can be attached to a geometry.
pub trait UserGeometryData: Sized + Send + Sync + 'static {}

impl<T> UserGeometryData for T where T: Sized + Send + Sync + 'static {}

/// User-defined data for a geometry.
///
/// This contains the pointer to the user-defined data and the type ID of the
/// user-defined data (which is used to check the type when getting the data).
#[derive(Debug, Clone)]
pub(crate) struct GeometryUserData {
    /// Pointer to the user-defined data.
    pub data: *mut std::os::raw::c_void,
    /// Type ID of the user-defined data.
    pub type_id: TypeId,
}

/// Payloads for user-defined callbacks of a geometry of kind
/// [`GeometryKind::USER`].
#[derive(Debug, Clone)]
pub(crate) struct UserGeometryPayloads {
    /// Payload for the [`UserGeometry::set_intersect_function`] call.
    pub intersect_fn: *mut std::os::raw::c_void,
    /// Payload for the [`UserGeometry::set_occluded_function`] call.
    pub occluded_fn: *mut std::os::raw::c_void,
    /// Payload for the [`UserGeometry::set_bounds_function`] call.
    pub bounds_fn: *mut std::os::raw::c_void,
}

/// Payloads for subdivision callbacks of a geometry of kind
/// [`GeometryKind::SUBDIVISION`].
#[derive(Debug, Clone)]
pub(crate) struct SubdivisionGeometryPayloads {
    /// Payload for the [`SubdivisionGeometry::set_vertex_function`] call.
    pub displacement_fn: *mut std::os::raw::c_void,
}

impl Default for UserGeometryPayloads {
    fn default() -> Self {
        Self {
            intersect_fn: ptr::null_mut(),
            occluded_fn: ptr::null_mut(),
            bounds_fn: ptr::null_mut(),
        }
    }
}

impl Default for SubdivisionGeometryPayloads {
    fn default() -> Self {
        Self {
            displacement_fn: ptr::null_mut(),
        }
    }
}

/// User-defined data for a geometry.
///
/// This contains also the payloads for different callbacks, which makes it
/// possible to pass Rust closures to Embree.
#[derive(Debug, Clone)]
pub(crate) struct GeometryData {
    /// User-defined data.
    pub user_data: Option<GeometryUserData>,
    /// Payload for the [`Geometry::set_intersect_filter_function`] call.
    pub intersect_filter_fn: *mut std::os::raw::c_void,
    /// Payload for the [`Geometry::set_occluded_filter_function`] call.
    pub occluded_filter_fn: *mut std::os::raw::c_void,
    /// Payloads only used for user geometry.
    pub user_fns: Option<UserGeometryPayloads>,
    /// Payloads only used for subdivision geometry.
    pub subdivision_fns: Option<SubdivisionGeometryPayloads>,
}

impl Default for GeometryData {
    fn default() -> Self {
        Self {
            user_data: None,
            intersect_filter_fn: ptr::null_mut(),
            occluded_filter_fn: ptr::null_mut(),
            user_fns: None,
            subdivision_fns: None,
        }
    }
}

/// Wrapper around an Embree geometry object.
///
/// A new geometry is created using [`Device::create_geometry`] or
/// new methods of different geometry types. Depending on the geometry type,
/// different buffers must be bound (e.g. using [`Geometry::set_buffer`]) to set
/// up the geometry data. In most cases, binding of a vertex and index buffer is
/// required. The number of primitives and vertices of that geometry is
/// typically inferred from the size of these bound buffers.
///
/// Changes to the geometry always must be committed using the
/// [`Geometry::commit`] call before using the geometry. After committing, a
/// geometry is not yet included in any scene. A geometry can be added to a
/// scene by using the [`Scene::attach_geometry`](crate::Scene::attach_geometry)
/// function (to automatically assign a geometry ID) or using the
/// [`Scene::attach_geometry_by_id`](crate::Scene::attach_geometry_by_id)
/// function (to specify the geometry ID manually). A geometry can get attached
/// to multiple scenes.
///
/// All geometry types support multi-segment motion blur with an arbitrary
/// number of equidistant time steps (in the range of 2 to 129) inside a user
/// specified time range. Each geometry can have a different number of time
/// steps and a different time range. The motion blur geometry is defined by
/// linearly interpolating the geometries of neighboring time steps. To
/// construct a motion blur geometry, first the number of time steps of the
/// geometry must be specified using [`Geometry::set_time_step_count`], and then
/// a vertex buffer for each time step must be bound, e.g. using the
/// [`Geometry::set_buffer`] function. Optionally, a time range defining the
/// start (and end time) of the first (and last) time step can be set using the
/// rtcSetGeometryTimeRange function. This feature will also allow geometries to
/// appear and disappear during the camera shutter time if the time range is a
/// sub range of [0,1].
///
/// The API supports per-geometry filter callback functions (see
/// [`Geometry::set_intersect_filter_function`]
/// and set_occluded_filter_function) that are invoked for each intersection
/// found during the Scene::intersect or Scene::occluded calls. The former ones
/// are called geometry intersection filter functions, the latter ones geometry
/// occlusion filter functions. These filter functions are designed to be used
/// to ignore intersections outside of a user- defined silhouette of a
/// primitive, e.g. to model tree leaves using transparency textures
///
/// It does not own the buffers that are bound to it, but it does own the
/// geometry object itself.
#[derive(Debug)]
pub struct Geometry<'buf> {
    pub(crate) device: Device,
    pub(crate) handle: RTCGeometry,
    kind: GeometryKind,
    /// Buffers that are attached to this geometry.
    attachments: Arc<Mutex<HashMap<BufferUsage, Vec<AttachedBuffer<'buf>>>>>,
    /// Data associated with this geometry.
    data: Arc<Mutex<GeometryData>>,
}

impl<'buf> Clone for Geometry<'buf> {
    fn clone(&self) -> Self {
        unsafe {
            rtcRetainGeometry(self.handle);
        }
        Self {
            device: self.device.clone(),
            handle: self.handle,
            kind: self.kind,
            attachments: self.attachments.clone(),
            data: self.data.clone(),
        }
    }
}

impl<'buf> Drop for Geometry<'buf> {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseGeometry(self.handle);
        }
    }
}

impl<'buf> Geometry<'buf> {
    /// Creates a new geometry object.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use embree::{Device, Geometry, GeometryKind};
    ///
    /// let device = Device::new().unwrap();
    /// let geometry = Geometry::new(&device, GeometryKind::TRIANGLE).unwrap();
    /// ```
    ///
    /// or use the [`Device::create_geometry`] method:
    ///
    /// ```no_run
    /// use embree::{Device, GeometryKind};
    ///
    /// let device = Device::new().unwrap();
    /// let geometry = device.create_geometry(GeometryKind::TRIANGLE).unwrap();
    /// ```
    pub fn new<'dev>(device: &'dev Device, kind: GeometryKind) -> Result<Geometry<'buf>, Error> {
        let handle = unsafe { rtcNewGeometry(device.handle, kind) };
        if handle.is_null() {
            Err(device.get_error())
        } else {
            let data = Arc::new(Mutex::new(GeometryData {
                user_data: None,
                intersect_filter_fn: ptr::null_mut(),
                occluded_filter_fn: ptr::null_mut(),
                user_fns: if kind == GeometryKind::USER {
                    Some(UserGeometryPayloads {
                        intersect_fn: ptr::null_mut(),
                        occluded_fn: ptr::null_mut(),
                        bounds_fn: ptr::null_mut(),
                    })
                } else {
                    None
                },
                subdivision_fns: if kind == GeometryKind::SUBDIVISION {
                    Some(SubdivisionGeometryPayloads {
                        displacement_fn: ptr::null_mut(),
                    })
                } else {
                    None
                },
            }));
            unsafe {
                rtcSetGeometryUserData(
                    handle,
                    Arc::into_raw(data.clone()) as *mut std::os::raw::c_void,
                );
            }
            Ok(Geometry {
                device: device.clone(),
                handle,
                kind,
                attachments: Arc::new(Mutex::new(HashMap::default())),
                data,
            })
        }
    }

    /// Disables the geometry.
    ///
    /// A disabled geometry is not rendered. Each geometry is enabled by
    /// default at construction time.
    /// After disabling a geometry, the scene containing that geometry must
    /// be committed using rtcCommitScene for the change to have effect.
    pub fn disable(&self) {
        unsafe {
            rtcDisableGeometry(self.handle);
        }
    }

    /// Enables the geometry.
    ///
    /// Only enabled geometries are rendered. Each geometry is enabled by
    /// default at construction time.
    ///
    /// After enabling a geometry, the scene containing that geometry must be
    /// committed using [`Geometry::commit`] for the change to have effect.
    pub fn enable(&self) {
        unsafe {
            rtcEnableGeometry(self.handle);
        }
    }

    /// Returns the raw Embree geometry handle.
    ///
    /// # Safety
    ///
    /// Use this function only if you know what you are doing. The returned
    /// handle is a raw pointer to an Embree reference-counted object. The
    /// reference count is not increased by this function, so the caller must
    /// ensure that the handle is not used after the geometry object is
    /// destroyed.
    pub unsafe fn handle(&self) -> RTCGeometry { self.handle }

    /// Checks if the vertex attribute is allowed for the geometry.
    ///
    /// This function do not check if the slot of the vertex attribute.
    fn check_vertex_attribute(&self) -> Result<(), Error> {
        match self.kind {
            GeometryKind::GRID | GeometryKind::USER | GeometryKind::INSTANCE => {
                eprint!(
                    "Vertex attribute not allowed for geometries of type {:?}!",
                    self.kind
                );
                Err(Error::INVALID_OPERATION)
            }
            _ => Ok(()),
        }
    }

    /// Binds a view of a buffer to the geometry.
    ///
    /// The buffer must be valid for the lifetime of the geometry. The buffer is
    /// provided as a [`BufferSlice`], which is a view into a buffer object.
    /// See the documentation of [`BufferSlice`] for more information.
    ///
    /// Under the hood, function call [`rtcSetGeometryBuffer`] is used to bind
    /// [`BufferSlice::Buffer`] or [`BufferSlice::GeometryLocal`] to the
    /// geometry, and [`rtcSetSharedGeometryBuffer`] is used to bind
    /// [`BufferSlice::User`].
    ///
    /// # Arguments
    ///
    /// * `usage` - The usage of the buffer.
    ///
    /// * `slot` - The slot to bind the buffer to. If the provided slot is
    ///   already bound to a buffer,
    ///  the old bound buffer will be overwritten with the new one.
    ///
    /// * `format` - The format of the buffer.
    ///
    /// * `slice` - The buffer slice to bind.
    ///
    /// * `stride` - The stride of the elements in the buffer. Must be a
    ///   multiple of 4.
    ///
    /// * `count` - The number of elements in the buffer.
    pub fn set_buffer<'a>(
        &'a mut self,
        usage: BufferUsage,
        slot: u32,
        format: Format,
        slice: BufferSlice<'buf>,
        stride: usize,
        count: usize,
    ) -> Result<(), Error> {
        debug_assert!(stride % 4 == 0, "Stride must be a multiple of 4!");
        if usage == BufferUsage::VERTEX {
            self.check_vertex_attribute()?;
        }
        match slice {
            BufferSlice::Buffer {
                buffer,
                offset,
                size,
            } => {
                dbg!(
                    "Binding buffer slice to slot {}, offset {}, stride {}, count {}",
                    slot,
                    offset,
                    stride,
                    count
                );
                let mut attachments = self.attachments.lock().unwrap();
                let bindings = attachments.entry(usage).or_insert_with(Vec::new);
                match bindings.iter().position(|a| a.slot == slot) {
                    // If the slot is already bound, remove the old binding and
                    // replace it with the new one.
                    Some(i) => {
                        bindings.remove(i);
                        unsafe {
                            rtcSetGeometryBuffer(
                                self.handle,
                                usage,
                                slot,
                                format,
                                buffer.handle,
                                offset,
                                stride,
                                count,
                            )
                        };
                        bindings.push(AttachedBuffer {
                            slot,
                            source: BufferSlice::Buffer {
                                buffer,
                                offset,
                                size,
                            },
                            format,
                            stride,
                        });
                        Ok(())
                    }
                    // If the slot is not bound, just bind the new buffer.
                    None => {
                        unsafe {
                            rtcSetGeometryBuffer(
                                self.handle,
                                usage,
                                slot,
                                format,
                                buffer.handle,
                                offset,
                                stride,
                                count,
                            )
                        };
                        bindings.push(AttachedBuffer {
                            slot,
                            source: BufferSlice::Buffer {
                                buffer,
                                offset,
                                size,
                            },
                            format,
                            stride,
                        });
                        Ok(())
                    }
                }
            }
            BufferSlice::GeometryLocal { .. } => {
                eprint!("Sharing geometry local buffer is not allowed!");
                Err(Error::INVALID_ARGUMENT)
            }
            BufferSlice::User {
                ptr, offset, size, ..
            } => {
                let mut attachments = self.attachments.lock().unwrap();
                let bindings = attachments.entry(usage).or_insert_with(Vec::new);
                match bindings.iter().position(|a| a.slot == slot) {
                    // If the slot is already bound, remove the old binding and
                    // replace it with the new one.
                    Some(i) => {
                        bindings.remove(i);
                        unsafe {
                            rtcSetSharedGeometryBuffer(
                                self.handle,
                                usage,
                                slot,
                                format,
                                ptr.add(offset) as *mut _,
                                offset,
                                stride,
                                count,
                            );
                        };
                        bindings.push(AttachedBuffer {
                            slot,
                            source: BufferSlice::User {
                                ptr,
                                offset,
                                size,
                                marker: PhantomData,
                            },
                            format,
                            stride,
                        });
                        Ok(())
                    }
                    // If the slot is not bound, just bind the new buffer.
                    None => {
                        unsafe {
                            rtcSetSharedGeometryBuffer(
                                self.handle,
                                usage,
                                slot,
                                format,
                                ptr.add(offset) as *mut _,
                                offset,
                                stride,
                                count,
                            );
                        };
                        bindings.push(AttachedBuffer {
                            slot,
                            source: BufferSlice::User {
                                ptr,
                                offset,
                                size,
                                marker: PhantomData,
                            },
                            format,
                            stride,
                        });
                        Ok(())
                    }
                }
            }
        }
    }

    /// Creates a new [`Buffer`] and binds it as a specific attribute for this
    /// geometry.
    ///
    /// Analogous to [`rtcSetNewGeometryBuffer`](https://spec.oneapi.io/oneart/0.5-rev-1/embree-spec.html#rtcsetnewgeometrybuffer).
    ///
    /// The allocated buffer will be automatically over-allocated slightly when
    /// used as a [`BufferUsage::VERTEX`] buffer, where a requirement is
    /// that each buffer element should be readable using 16-byte SSE load
    /// instructions. This means that the buffer will be padded to a multiple of
    /// 16 bytes.
    ///
    /// The allocated buffer is managed internally and automatically released
    /// when the geometry is destroyed by Embree.
    ///
    /// # Arguments
    ///
    /// * `usage` - The usage of the buffer.
    ///
    /// * `slot` - The slot to bind the buffer to.
    ///
    /// * `format` - The format of the buffer items. See [`Format`] for more
    ///   information.
    ///
    /// * `count` - The number of items in the buffer.
    ///
    /// * `stride` - The stride of the buffer items. MUST be a multiple of 4.
    pub fn set_new_buffer(
        &mut self,
        usage: BufferUsage,
        slot: u32,
        format: Format,
        stride: usize,
        count: usize,
    ) -> Result<BufferSlice<'static>, Error> {
        debug_assert!(stride % 4 == 0, "Stride must be a multiple of 4!");
        if usage == BufferUsage::VERTEX_ATTRIBUTE {
            self.check_vertex_attribute()?;
        }
        {
            let mut attachments = self.attachments.lock().unwrap();
            let bindings = attachments.entry(usage).or_insert_with(Vec::new);
            if !bindings.iter().any(|a| a.slot == slot) {
                let raw_ptr = unsafe {
                    rtcSetNewGeometryBuffer(self.handle, usage, slot, format, stride, count)
                };
                if raw_ptr.is_null() {
                    Err(self.device.get_error())
                } else {
                    let slice = BufferSlice::GeometryLocal {
                        ptr: raw_ptr,
                        size: NonZeroUsize::new(count * stride).unwrap(),
                        marker: PhantomData,
                    };
                    bindings.push(AttachedBuffer {
                        slot,
                        source: slice,
                        format,
                        stride,
                    });
                    Ok(slice)
                }
            } else {
                eprint!("Buffer already attached to slot {}", slot);
                Err(Error::INVALID_ARGUMENT)
            }
        }
    }

    /// Returns the buffer bound to the given slot and usage.
    pub fn get_buffer(&self, usage: BufferUsage, slot: u32) -> Option<BufferSlice> {
        let attachments = self.attachments.lock().unwrap();
        attachments
            .get(&usage)
            .and_then(|v| v.iter().find(|a| a.slot == slot))
            .map(|a| a.source)
    }

    /// Marks a buffer slice bound to this geometry as modified.
    ///
    /// If a data buffer is changed by the application, this function must be
    /// called for the buffer to be updated in the geometry. Each buffer slice
    /// assigned to a buffer slot is initially marked as modified, thus this
    /// method needs to be called only when doing buffer modifications after the
    /// first [`Scene::commit`] call.
    pub fn update_buffer(&self, usage: BufferUsage, slot: u32) {
        unsafe {
            rtcUpdateGeometryBuffer(self.handle, usage, slot);
        }
    }

    /// Returns the type of geometry of this geometry.
    pub fn kind(&self) -> GeometryKind { self.kind }

    pub fn commit(&mut self) {
        unsafe {
            rtcCommitGeometry(self.handle);
        }
    }

    /// Sets the build quality for the geometry.
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
    pub fn set_build_quality(&mut self, quality: BuildQuality) {
        unsafe {
            rtcSetGeometryBuildQuality(self.handle, quality);
        }
    }

    /// Registers an intersection filter callback function for the geometry.
    ///
    /// Only a single callback function can be registered per geometry, and
    /// further invocations overwrite the previously set callback function.
    /// Unregister the callback function by calling
    /// [`Geometry::unset_intersect_filter_function`].
    ///
    /// The registered filter function is invoked for every hit encountered
    /// during the intersect-type ray queries and can accept or reject that
    /// hit. The feature can be used to define a silhouette for a primitive
    /// and reject hits that are outside the silhouette. E.g. a tree leaf
    /// could be modeled with an alpha texture that decides whether hit
    /// points lie inside or outside the leaf.
    ///
    /// If [`BuildQuality::HIGH`] is set, the filter functions may be called
    /// multiple times for the same primitive hit. Further, rays hitting
    /// exactly the edge might also report two hits for the same surface. For
    /// certain use cases, the application may have to work around this
    /// limitation by collecting already reported hits (geomID/primID pairs)
    /// and ignoring duplicates.
    ///
    /// The filter function callback of type [`RTCFilterFunctionN`] gets passed
    /// a number of arguments through the [`RTCFilterFunctionNArguments`]
    /// structure. The valid parameter of that structure points to an
    /// integer valid mask (0 means invalid and -1 means valid). The
    /// `geometryUserPtr` member is a user pointer optionally set per
    /// geometry through the [`Geometry::set_user_data`] function. The
    /// context member points to the intersection context passed to
    /// the ray query function. The ray parameter points to N rays in SOA layout
    /// (see `RayN`, `HitN`).
    /// The hit parameter points to N hits in SOA layout to test. The N
    /// parameter is the number of rays and hits in ray and hit. The hit
    /// distance is provided as the tfar value of the ray. If the hit
    /// geometry is instanced, the `instID` member of the ray is valid, and
    /// the ray and the potential hit are in object space.
    ///
    /// The filter callback function has the task to check for each valid ray
    /// whether it wants to accept or reject the corresponding hit. To
    /// reject a hit, the filter callback function just has to *write 0* to
    /// the integer valid mask of the corresponding ray. To accept the hit,
    /// it just has to *leave the valid mask set to -1*. The filter function
    /// is further allowed to change the hit and decrease the tfar value of the
    /// ray but it should not modify other ray data nor any inactive
    /// components of the ray or hit.
    ///
    /// When performing ray queries using [`Scene::intersect`], it is
    /// *guaranteed* that the packet size is 1 when the callback is invoked.
    /// When performing ray queries using the [`Scene::intersect4/8/16`]
    /// functions, it is not generally guaranteed that the ray packet size
    /// (and order of rays inside the packet) passed to the callback matches
    /// the initial ray packet. However, under some circumstances these
    /// properties are guaranteed, and whether this is the case can be
    /// queried using [`Device::get_property`]. When performing ray queries
    /// using the stream API such as [`Scene::intersect_stream_aos`],
    /// [`Scene::intersect1Mp`], [`Scene::intersect_stream_soa`], the order
    /// of rays and ray packet size of the callback function might change to
    /// either 1, 4, 8, or 16.
    ///
    /// For many usage scenarios, repacking and re-ordering of rays does not
    /// cause difficulties in implementing the callback function. However,
    /// algorithms that need to extend the ray with additional data must use
    /// the rayID component of the ray to identify the original ray to
    /// access the per-ray data.
    pub fn set_intersect_filter_function<F, D>(&mut self, filter: F)
    where
        D: UserGeometryData,
        F: for<'a> FnMut(&'a mut [i32], Option<&mut D>, &mut IntersectContext, RayN<'a>, HitN<'a>),
    {
        let mut geom_data = self.data.lock().unwrap();
        unsafe {
            let mut closure = filter;
            geom_data.intersect_filter_fn = &mut closure as *mut _ as *mut std::os::raw::c_void;
            rtcSetGeometryIntersectFilterFunction(
                self.handle,
                intersect_filter_function(&mut closure),
            );
        }
    }

    /// Unsets the intersection filter function for the geometry.
    pub fn unset_intersect_filter_function(&mut self) {
        unsafe {
            rtcSetGeometryIntersectFilterFunction(self.handle, None);
        }
    }

    /// Sets the occlusion filter for the geometry.
    ///
    /// Only a single callback function can be registered per geometry, and
    /// further invocations overwrite the previously set callback function.
    /// Unregister the callback function by calling
    /// [`Geometry::unset_occluded_filter_function`].
    ///
    /// The registered intersection filter function is invoked for every hit
    /// encountered during the occluded-type ray queries and can accept or
    /// reject that hit.
    ///
    /// The feature can be used to define a silhouette for a primitive and
    /// reject hits that are outside the silhouette. E.g. a tree leaf could
    /// be modeled with an alpha texture that decides whether hit points lie
    /// inside or outside the leaf. Please see the description of the
    /// [`Geometry::set_intersect_filter_function`] for a description of the
    /// filter callback function.
    pub fn set_occluded_filter_function<F, D>(&mut self, filter: F)
    where
        D: UserGeometryData,
        F: FnMut(&mut [i32], Option<&mut D>, &mut IntersectContext, RayN, HitN),
    {
        let mut geom_data = self.data.lock().unwrap();
        unsafe {
            let mut closure = filter;
            geom_data.occluded_filter_fn = &mut closure as *mut _ as *mut std::os::raw::c_void;
            rtcSetGeometryOccludedFilterFunction(
                self.handle,
                occluded_filter_function(&mut closure),
            );
        }
    }

    /// Unsets the occlusion filter function for the geometry.
    pub fn unset_occluded_filter_function(&mut self) {
        unsafe {
            rtcSetGeometryOccludedFilterFunction(self.handle, None);
        }
    }

    // TODO(yang): how to handle the closure? RTCPointQueryFunctionArguments has a
    // user pointer but we can't set it here, instead we can only set it in the
    // rtcPointQuery function which is attached to the scene. This requires the
    // user to call [`Scene::point_query`] first and then call
    // [`Geometry::set_point_query_function`] to set the closure. Or we can
    // make the closure a member of the [`GeometryData`] and set it here.

    /// Sets the point query callback function for a geometry.
    ///
    /// Only a single callback function can be registered per geometry and
    /// further invocations overwrite the previously set callback function.
    /// Unregister the callback function by calling
    /// [`Geometry::unset_point_query_function`].
    ///
    /// The registered callback function is invoked by rtcPointQuery for every
    /// primitive of the geometry that intersects the corresponding point query
    /// domain. The callback function of type `RTCPointQueryFunction` gets
    /// passed a number of arguments through the
    /// `RTCPointQueryFunctionArguments` structure. The query object is the
    /// original point query object passed into rtcPointQuery, us-
    /// rPtr is an arbitrary pointer to pass input into and store results of the
    /// callback function. The primID, geomID and context (see
    /// rtcInitPointQueryContext for details) can be used to identify the
    /// geometry data of the primitive. 122Embree API Reference
    /// A RTCPointQueryFunction can also be passed directly as an argument to
    /// rtcPointQuery. In this case the callback is invoked for all primitives
    /// in the scene that intersect the query domain. If a callback function
    /// is passed as an argument to rtcPointQuery and (a potentially
    /// different) callback function is set for a ge- ometry with
    /// rtcSetGeometryPointQueryFunction both callback functions are in-
    /// voked and the callback function passed to rtcPointQuery will be called
    /// before the geometry specific callback function.
    /// If instancing is used, the parameter simliarityScale indicates whether
    /// the current instance transform (top element of the stack in context)
    /// is a similarity transformation or not. Similarity transformations
    /// are composed of translation, rotation and uniform scaling and if a
    /// matrix M defines a similarity transformation, there is a scaling
    /// factor D such that for all x,y: dist(Mx, My) = D * dist(x,
    /// y). In this case the parameter scalingFactor is this scaling factor D
    /// and other- wise it is 0. A valid similarity scale (similarityScale >
    /// 0) allows to compute distance information in instance space and
    /// scale the distances into world space (for example, to update the
    /// query radius, see below) by dividing the instance space distance
    /// with the similarity scale. If the current instance transform is not
    /// a similarity transform (similarityScale is 0), the distance computation
    /// has to be performed in world space to ensure correctness. In this
    /// case the instance to world transformations given with the context
    /// should be used to transform the primitive data into world space.
    /// Otherwise, the query location can be trans- formed into instance
    /// space which can be more efficient. If there is no instance
    /// transform, the similarity scale is 1.
    /// The callback function will potentially be called for primitives outside
    /// the query domain for two reasons: First, the callback is invoked for
    /// all primitives inside a BVH leaf node since no geometry data of
    /// primitives is determined internally and therefore individual
    /// primitives are not culled (only their (aggregated) bounding boxes).
    /// Second, in case non similarity transformations are used, the
    /// resulting ellipsoidal query domain (in instance space) is approximated
    /// by its axis aligned bounding box internally and therefore inner
    /// nodes that do not intersect the original domain might intersect the
    /// approximative bounding box which results in unnecessary callbacks.
    /// In any case, the callbacks are conservative, i.e. if a primitive is
    /// inside the query domain a callback will be invoked but the reverse
    /// is not necessarily true.
    /// For efficiency, the radius of the query object can be decreased (in
    /// world space) inside the callback function to improve culling of
    /// geometry during BVH traversal. If the query radius was updated, the
    /// callback function should return true to issue an update of internal
    /// traversal information. Increasing the radius or modifying
    /// the time or position of the query results in undefined behaviour.
    /// Within the callback function, it is safe to call rtcPointQuery again,
    /// for ex- ample when implementing instancing manually. In this case
    /// the instance trans- formation should be pushed onto the stack in
    /// context. Embree will internally compute the point query information
    /// in instance space using the top element of the stack in context when
    /// rtcPointQuery is called. For a reference implementation of a closest
    /// point traversal of triangle meshes using instancing and user defined
    /// instancing see the tutorial [ClosestPoint].
    pub unsafe fn set_point_query_function(&mut self, query_fn: RTCPointQueryFunction) {
        rtcSetGeometryPointQueryFunction(self.handle, query_fn);
    }

    /// Unsets the point query function for the geometry.
    pub fn unset_point_query_function(&mut self) {
        unsafe {
            rtcSetGeometryPointQueryFunction(self.handle, None);
        }
    }

    /// Sets the tessellation rate for a subdivision mesh or flat curves.
    ///
    /// For curves, the tessellation rate specifies the number of ray-facing
    /// quads per curve segment. For subdivision surfaces, the tessellation
    /// rate specifies the number of quads along each edge.
    pub fn set_tessellation_rate(&mut self, rate: f32) {
        match self.kind {
            GeometryKind::SUBDIVISION
            | GeometryKind::FLAT_LINEAR_CURVE
            | GeometryKind::FLAT_BEZIER_CURVE
            | GeometryKind::ROUND_LINEAR_CURVE
            | GeometryKind::ROUND_BEZIER_CURVE => unsafe {
                rtcSetGeometryTessellationRate(self.handle, rate);
            },
            _ => panic!(
                "Geometry::set_tessellation_rate is only supported for subdivision meshes and \
                 flat curves"
            ),
        }
    }

    /// Sets the mask for the geometry.
    ///
    /// This geometry mask is used together with the ray mask stored inside the
    /// mask field of the ray. The primitives of the geometry are hit by the ray
    /// only if the bitwise and operation of the geometry mask with the ray mask
    /// is not 0.
    /// This feature can be used to disable selected geometries for specifically
    /// tagged rays, e.g. to disable shadow casting for certain geometries.
    ///
    /// Ray masks are disabled in Embree by default at compile time, and can be
    /// enabled through the `EMBREE_RAY_MASK` parameter in CMake. One can query
    /// whether ray masks are enabled by querying the
    /// [`DeviceProperty::RAY_MASK_SUPPORTED`] device property using
    /// [`Device::get_property`].
    pub fn set_mask(&mut self, mask: u32) {
        unsafe {
            rtcSetGeometryMask(self.handle, mask);
        }
    }

    /// Sets the number of time steps for multi-segment motion blur for the
    /// geometry.
    ///
    /// For triangle meshes, quad meshes, curves, points, and subdivision
    /// geometries, the number of time steps directly corresponds to the
    /// number of vertex buffer slots available [`BufferUsage::VERTEX`].
    ///
    /// For instance geometries, a transformation must be specified for each
    /// time step (see [`Geometry::set_transform`]).
    ///
    /// For user geometries, the registered bounding callback function must
    /// provide a bounding box per primitive and time step, and the
    /// intersection and occlusion callback functions should properly
    /// intersect the motion-blurred geometry at the ray time.
    pub fn set_time_step_count(&mut self, count: u32) {
        unsafe {
            rtcSetGeometryTimeStepCount(self.handle, count);
        }
    }

    /// Sets the time range for a motion blur geometry.
    ///
    /// The time range is defined relative to the camera shutter interval [0,1]
    /// but it can be arbitrary. Thus the `start` time can be smaller,
    /// equal, or larger 0, indicating a geometry whose animation definition
    /// start before, at, or after the camera shutter opens.
    /// Similar the `end` time can be smaller, equal, or larger than 1,
    /// indicating a geometry whose animation definition ends after, at, or
    /// before the camera shutter closes. The `start` time has to be smaller
    /// or equal to the `end` time.
    ///
    /// The default time range when this function is not called is the entire
    /// camera shutter [0,1]. For best performance at most one time segment
    /// of the piece wise linear definition of the motion should fall
    /// outside the shutter window to the left and to the right. Thus do not
    /// set the `start` time or `end` time too far outside the
    /// [0,1] interval for best performance.
    ///
    /// This time range feature will also allow geometries to appear and
    /// disappear during the camera shutter time if the specified time range
    /// is a sub range of [0,1].
    ///
    /// Please also have a look at the [`Geometry::set_time_step_count`] to
    /// see how to define the time steps for the specified time range.
    pub fn set_time_range(&mut self, start: f32, end: f32) {
        unsafe {
            rtcSetGeometryTimeRange(self.handle, start, end);
        }
    }

    /// Sets the user-defined data pointer of the geometry.
    ///
    /// The user data pointer is intended to be pointing to the application's
    /// representation of the geometry, and is passed to various callback
    /// functions.
    ///
    /// The application can use this pointer inside the callback functions to
    /// access its geometry representation.
    pub fn set_user_data<D>(&mut self, user_data: &mut D)
    where
        D: UserGeometryData,
    {
        let mut geom_data = self.data.lock().unwrap();
        geom_data.user_data = Some(GeometryUserData {
            data: user_data as *mut D as *mut std::os::raw::c_void,
            type_id: TypeId::of::<D>(),
        });
        unsafe {
            rtcSetGeometryUserData(
                self.handle,
                geom_data.deref_mut() as *mut GeometryData as *mut _,
            );
        }
    }

    /// Returns the user data pointer of the geometry.
    pub fn get_user_data<D>(&self) -> Option<&mut D>
    where
        D: UserGeometryData,
    {
        unsafe {
            let ptr = rtcGetGeometryUserData(self.handle) as *mut GeometryData;
            if ptr.is_null() {
                None
            } else {
                match (*ptr).user_data.as_mut() {
                    None => None,
                    Some(user_data @ GeometryUserData { .. }) => {
                        if user_data.type_id == TypeId::of::<D>() {
                            Some(&mut *(user_data.data as *mut D))
                        } else {
                            None
                        }
                    }
                }
            }
        }
    }

    /// Sets the number of vertex attributes of the geometry.
    ///
    /// This function sets the number of slots for vertex attributes buffers
    /// (BufferUsage::VERTEX_ATTRIBUTE) that can be used for the specified
    /// geometry.
    ///
    /// Only supported by triangle meshes, quad meshes, curves, points, and
    /// subdivision geometries.
    ///
    /// # Arguments
    ///
    /// * `count` - The number of vertex attribute slots.
    pub fn set_vertex_attribute_count(&mut self, count: u32) {
        match self.kind {
            GeometryKind::GRID | GeometryKind::USER | GeometryKind::INSTANCE => {
                eprint!(
                    "Vertex attribute not allowed for geometries of type {:?}!",
                    self.kind
                );
            }
            _ => {
                // Update the vertex attribute count.
                unsafe {
                    rtcSetGeometryVertexAttributeCount(self.handle, count);
                }
            }
        }
    }

    /// Binds a vertex attribute to a topology of the geometry.
    ///
    /// This function binds a vertex attribute buffer slot to a topology for the
    /// specified subdivision geometry. Standard vertex buffers are always bound
    /// to the default topology (topology 0) and cannot be bound
    /// differently. A vertex attribute buffer always uses the topology it
    /// is bound to when used in the `rtcInterpolate` and `rtcInterpolateN`
    /// calls.
    ///
    /// A topology with ID `i` consists of a subdivision mode set through
    /// `Geometry::set_subdivision_mode` and the index buffer bound to the index
    /// buffer slot `i`. This index buffer can assign indices for each face of
    /// the subdivision geometry that are different to the indices of the
    /// default topology. These new indices can for example be used to
    /// introduce additional borders into the subdivision mesh to map
    /// multiple textures onto one subdivision geometry.
    pub fn set_vertex_attribute_topology(&self, vertex_attribute_id: u32, topology_id: u32) {
        unsafe {
            rtcSetGeometryVertexAttributeTopology(self.handle, vertex_attribute_id, topology_id);
        }
    }

    /// Smoothly interpolates per-vertex data over the geometry.
    ///
    /// This interpolation is supported for triangle meshes, quad meshes, curve
    /// geometries, and subdivision geometries. Apart from interpolating the
    /// vertex at- tribute itself, it is also possible to get the first and
    /// second order derivatives of that value. This interpolation ignores
    /// displacements of subdivision surfaces and always interpolates the
    /// underlying base surface.
    ///
    /// Interpolated values are written to `args.p`, `args.dp_du`, `args.dp_dv`,
    /// `args.ddp_du_du`, `args.ddp_dv_dv`, and `args.ddp_du_dv`. Set them to
    /// `None` if you do not need to interpolate them.
    ///
    /// All output arrays must be padded to 16 bytes.
    pub fn interpolate(&self, input: InterpolateInput, output: &mut InterpolateOutput) {
        let args = RTCInterpolateArguments {
            geometry: self.handle,
            primID: input.prim_id,
            u: input.u,
            v: input.v,
            bufferType: input.usage,
            bufferSlot: input.slot,
            P: output
                .p_mut()
                .map(|p| p.as_mut_ptr())
                .unwrap_or(ptr::null_mut()),
            dPdu: output
                .dp_du_mut()
                .map(|p| p.as_mut_ptr())
                .unwrap_or(ptr::null_mut()),
            dPdv: output
                .dp_dv_mut()
                .map(|p| p.as_mut_ptr())
                .unwrap_or(ptr::null_mut()),
            ddPdudu: output
                .ddp_du_du_mut()
                .map(|p| p.as_mut_ptr())
                .unwrap_or(ptr::null_mut()),
            ddPdvdv: output
                .ddp_dv_dv_mut()
                .map(|p| p.as_mut_ptr())
                .unwrap_or(ptr::null_mut()),
            ddPdudv: output
                .ddp_du_dv_mut()
                .map(|p| p.as_mut_ptr())
                .unwrap_or(ptr::null_mut()),
            valueCount: output.value_count(),
        };
        unsafe {
            rtcInterpolate(&args as _);
        }
    }

    /// Performs N interpolations of vertex attribute data.
    ///
    /// Similar to [`Geometry::interpolate`], but performs N many interpolations
    /// at once. It additionally gets an array of u/v coordinates
    /// [`InterpolateNInput::u/v`]and a valid mask
    /// [`InterpolateNInput::valid`] that specifies which of these
    /// coordinates are valid. The valid mask points to `n` integers, and a
    /// value of -1 denotes valid and 0 invalid.
    ///
    /// If [`InterpolateNInput::valid`] is `None`, all coordinates are
    /// assumed to be valid.
    ///
    /// The destination arrays are filled in structure of array (SOA) layout.
    /// The value [`InterpolateNInput::n`] must be divisible by 4.
    ///
    /// All changes to that geometry must be properly committed.
    pub fn interpolate_n(&self, input: InterpolateNInput, output: &mut InterpolateOutput) {
        assert_eq!(input.n % 4, 0, "N must be a multiple of 4!");
        let args = RTCInterpolateNArguments {
            geometry: self.handle,
            N: input.n,
            valid: input
                .valid
                .as_ref()
                .map(|v| v.as_ptr() as *const _)
                .unwrap_or(ptr::null()),
            primIDs: input.prim_id.as_ptr(),
            u: input.u.as_ptr(),
            v: input.v.as_ptr(),
            bufferType: input.usage,
            bufferSlot: input.slot,
            P: output
                .p_mut()
                .map(|p| p.as_mut_ptr())
                .unwrap_or(ptr::null_mut()),
            dPdu: output
                .dp_du_mut()
                .map(|p| p.as_mut_ptr())
                .unwrap_or(ptr::null_mut()),
            dPdv: output
                .dp_dv_mut()
                .map(|p| p.as_mut_ptr())
                .unwrap_or(ptr::null_mut()),
            ddPdudu: output
                .ddp_du_du_mut()
                .map(|p| p.as_mut_ptr())
                .unwrap_or(ptr::null_mut()),
            ddPdvdv: output
                .ddp_dv_dv_mut()
                .map(|p| p.as_mut_ptr())
                .unwrap_or(ptr::null_mut()),
            ddPdudv: output
                .ddp_du_dv_mut()
                .map(|p| p.as_mut_ptr())
                .unwrap_or(ptr::null_mut()),
            valueCount: output.value_count(),
        };
        unsafe {
            rtcInterpolateN(&args as _);
        }
    }

    /// Sets a callback to query the bounding box of user-defined primitives.
    ///
    /// Only a single callback function can be registered per geometry, and
    /// further invocations overwrite the previously set callback function.
    ///
    /// Unregister the callback function by calling
    /// [`Geometry::unset_bounds_function`].
    ///
    /// The registered bounding box callback function is invoked to calculate
    /// axis- aligned bounding boxes of the primitives of the user-defined
    /// geometry during spatial acceleration structure construction.
    ///
    /// The arguments of the callback closure are:
    ///
    /// - a mutable reference to the user data of the geometry
    ///
    /// - the ID of the primitive to calculate the bounds for
    ///
    /// - the time step at which to calculate the bounds
    ///
    /// - a mutable reference to the bounding box where the result should be
    ///   written to
    ///
    /// In a typical usage scenario one would store a pointer to the internal
    /// representation of the user geometry object using
    /// [`Geometry::set_user_data`]. The callback function can then read
    /// that pointer from the `geometryUserPtr` field and calculate the
    /// proper bounding box for the requested primitive and time, and store
    /// that bounding box to the destination structure (`bounds_o` member).
    pub fn set_bounds_function<F, D>(&mut self, bounds: F)
    where
        D: UserGeometryData,
        F: FnMut(Option<&mut D>, u32, u32, &mut Bounds),
    {
        match self.kind {
            GeometryKind::USER => unsafe {
                let mut geom_data = self.data.lock().unwrap();
                let mut closure = bounds;
                geom_data.user_fns.as_mut().unwrap().bounds_fn =
                    &mut closure as *mut _ as *mut std::os::raw::c_void;
                rtcSetGeometryBoundsFunction(
                    self.handle,
                    bounds_function(&mut closure),
                    ptr::null_mut(),
                );
            },
            _ => panic!("Only user geometries can have a bounds function!"),
        }
    }

    /// Unsets the callback to calculate the bounding box of user-defined
    /// geometry.
    pub fn unset_bounds_function(&mut self) {
        match self.kind {
            GeometryKind::USER => unsafe {
                rtcSetGeometryBoundsFunction(self.handle, None, ptr::null_mut());
            },
            _ => panic!("Only user geometries can have a bounds function!"),
        }
    }

    /// Sets the callback function to intersect a user geometry.
    ///
    /// Only a single callback function can be registered per geometry and
    /// further invocations overwrite the previously set callback function.
    /// Unregister the callback function by calling
    /// [`Geometry::unset_intersect_function`].
    ///
    /// The registered callback function is invoked by intersect-type ray
    /// queries to calculate the intersection of a ray packet of variable
    /// size with one user-defined primitive. The callback function of type
    /// [`RTCIntersectFunctionN`] gets passed a number of arguments through
    /// the [`RTCIntersectFunctionNArguments`] structure. The value N
    /// specifies the ray packet size, valid points to an array of
    /// integers that specify whether the corresponding ray is valid (-1) or
    /// invalid (0), the `geometryUserPtr` member points to the geometry
    /// user data previously set through [`Geometry::set_user_data`], the
    /// context member points to the intersection context passed to the ray
    /// query, the rayhit member points to a ray and hit packet of variable
    /// size N, and the geomID and primID member identifies the geometry ID
    /// and primitive ID of the primitive to intersect. The ray component of
    /// the rayhit structure contains valid data, in particular
    /// the tfar value is the current closest hit distance found. All data
    /// inside the hit component of the rayhit structure are undefined and
    /// should not be read by the function.
    /// The task of the callback function is to intersect each active ray from
    /// the ray packet with the specified user primitive. If the
    /// user-defined primitive is missed by a ray of the ray packet, the
    /// function should return without modifying the ray or hit. If an
    /// intersection of the user-defined primitive with the ray was found in
    /// the valid range (from tnear to tfar), it should update the hit distance
    /// of the ray (tfar member) and the hit (u, v, Ng, instID, geomID,
    /// primID members). In particular, the currently intersected instance
    /// is stored in the instID field of the intersection context, which
    /// must be deep copied into the instID member of the hit.
    ///
    /// As a primitive might have multiple intersections with a ray, the
    /// intersection filter function needs to be invoked by the user
    /// geometry intersection callback for each encountered intersection, if
    /// filtering of intersections is desired. This can be achieved through
    /// the rtcFilterIntersection call. Within the user geometry intersect
    /// function, it is safe to trace new rays and create new scenes and
    /// geometries. When performing ray queries using rtcIntersect1, it is
    /// guaranteed that the packet size is 1 when the callback is invoked.
    /// When performing ray queries using the rtcIntersect4/8/16 functions,
    /// it is not generally guaranteed that the ray packet size (and order
    /// of rays inside the packet) passed to the callback matches
    /// the initial ray packet. However, under some circumstances these
    /// properties are guaranteed, and whether this is the case can be
    /// queried using rtcGetDevice- Property. When performing ray queries
    /// using the stream API such as rtcIntersect1M, rtcIntersect1Mp,
    /// rtcIntersectNM, or rtcIntersectNp the or- der of rays and ray packet
    /// size of the callback function might change to either 1, 4, 8, or 16.
    /// For many usage scenarios, repacking and re-ordering of rays does not
    /// cause difficulties in implementing the callback function. However,
    /// algorithms that need to extend the ray with additional data must use
    /// the rayID component of the ray to identify the original ray to
    /// access the per-ray data.
    pub fn set_intersect_function<F, D>(&mut self, intersect: F)
    where
        D: UserGeometryData,
        F: for<'a> FnMut(
            &'a mut [i32],
            Option<&mut D>,
            u32,
            u32,
            &mut IntersectContext,
            RayHitN<'a>,
        ),
    {
        match self.kind {
            GeometryKind::USER => unsafe {
                let mut geom_data = self.data.lock().unwrap();
                let mut closure = intersect;
                geom_data.user_fns.as_mut().unwrap().intersect_fn =
                    &mut closure as *mut _ as *mut std::os::raw::c_void;
                rtcSetGeometryIntersectFunction(self.handle, intersect_function(&mut closure));
            },
            _ => panic!("Only user geometries can have an intersect function!"),
        }
    }

    /// Unsets the callback to intersect user-defined geometry.
    pub fn unset_intersect_function(&mut self) {
        match self.kind {
            GeometryKind::USER => unsafe {
                rtcSetGeometryIntersectFunction(self.handle, None);
            },
            _ => panic!("Only user geometries can have an intersect function!"),
        }
    }

    /// Sets the callback function to occlude a user geometry.
    ///
    /// Similar to [`Geometry::set_intersect_function`], but for occlusion
    /// queries.
    pub fn set_occluded_function<F, D>(&mut self, occluded: F)
    where
        D: UserGeometryData,
        F: for<'a> FnMut(&'a mut [i32], Option<&mut D>, u32, u32, &mut IntersectContext, RayN<'a>),
    {
        match self.kind {
            GeometryKind::USER => {
                let mut geom_data = self.data.lock().unwrap();
                let mut closure = occluded;
                geom_data.user_fns.as_mut().unwrap().occluded_fn =
                    &mut closure as *mut _ as *mut std::os::raw::c_void;
                unsafe {
                    rtcSetGeometryOccludedFunction(self.handle, occluded_function(&mut closure))
                };
            }
            _ => panic!("Only user geometries can have an occluded function!"),
        }
    }

    /// Unsets the callback to occlude user-defined geometry.
    pub fn unset_occluded_function(&mut self) {
        match self.kind {
            GeometryKind::USER => unsafe {
                rtcSetGeometryOccludedFunction(self.handle, None);
            },
            _ => panic!("Only user geometries can have an occluded function!"),
        }
    }

    /// Sets the number of primitives of a user-defined geometry.
    pub fn set_primitive_count(&mut self, count: u32) {
        match self.kind {
            GeometryKind::USER => unsafe {
                rtcSetGeometryUserPrimitiveCount(self.handle, count);
            },
            _ => panic!("Only user geometries can have a primitive count!"),
        }
    }

    /// Set the subdivision mode for the topology of the specified subdivision
    /// geometry.
    ///
    /// The subdivision modes can be used to force linear interpolation for
    /// certain parts of the subdivision mesh:
    ///
    /// * [`RTCSubdivisionMode::NO_BOUNDARY`]: Boundary patches are ignored.
    /// This way each rendered patch has a full set of control vertices.
    ///
    /// * [`RTCSubdivisionMode::SMOOTH_BOUNDARY`]: The sequence of boundary
    /// control points are used to generate a smooth B-spline boundary curve
    /// (default mode).
    ///
    /// * [`RTCSubdivisionMode::PIN_CORNERS`]: Corner vertices are pinned to
    /// their location during subdivision.
    ///
    /// * [`RTCSubdivisionMode::PIN_BOUNDARY`]: All vertices at the border are
    /// pinned to their location during subdivision. This way the boundary is
    /// interpolated linearly. This mode is typically used for texturing to also
    /// map texels at the border of the texture to the mesh.
    ///
    /// * [`RTCSubdivisionMode::PIN_ALL`]: All vertices at the border are pinned
    /// to their location during subdivision. This way all patches are linearly
    /// interpolated.
    pub fn set_subdivision_mode(&self, topology_id: u32, mode: SubdivisionMode) {
        match self.kind {
            GeometryKind::SUBDIVISION => unsafe {
                rtcSetGeometrySubdivisionMode(self.handle, topology_id, mode)
            },
            _ => panic!("Only subdivision geometries can have a subdivision mode!"),
        }
    }

    /// Sets the number of topologies of a subdivision geometry.
    ///
    /// The number of topologies of a subdivision geometry must be greater
    /// or equal to 1.
    ///
    /// To use multiple topologies, first the number of topologies must be
    /// specified, then the individual topologies can be configured using
    /// [`Geometry::set_subdivision_mode`] and by setting an index buffer
    /// ([`BufferUsage::INDEX`]) using the topology ID as the buffer slot.
    pub fn set_topology_count(&mut self, count: u32) {
        match self.kind {
            GeometryKind::SUBDIVISION => unsafe {
                rtcSetGeometryTopologyCount(self.handle, count);
            },
            _ => panic!("Only subdivision geometries can have multiple topologies!"),
        }
    }

    /// Returns the first half edge of a face.
    ///
    /// This function can only be used for subdivision meshes. As all topologies
    /// of a subdivision geometry share the same face buffer the function does
    /// not depend on the topology ID.
    pub fn get_first_half_edge(&self, face_id: u32) -> u32 {
        match self.kind {
            GeometryKind::SUBDIVISION => unsafe {
                rtcGetGeometryFirstHalfEdge(self.handle, face_id)
            },
            _ => panic!("Only subdivision geometries can have half edges!"),
        }
    }

    /// Returns the face of some half edge.
    ///
    /// This function can only be used for subdivision meshes. As all topologies
    /// of a subdivision geometry share the same face buffer the function does
    /// not depend on the topology ID.
    pub fn get_face(&self, half_edge_id: u32) -> u32 {
        match self.kind {
            GeometryKind::SUBDIVISION => unsafe { rtcGetGeometryFace(self.handle, half_edge_id) },
            _ => panic!("Only subdivision geometries can have half edges!"),
        }
    }

    /// Returns the next half edge of some half edge.
    ///
    /// This function can only be used for subdivision meshes. As all topologies
    /// of a subdivision geometry share the same face buffer the function does
    /// not depend on the topology ID.
    pub fn get_next_half_edge(&self, half_edge_id: u32) -> u32 {
        match self.kind {
            GeometryKind::SUBDIVISION => unsafe {
                rtcGetGeometryNextHalfEdge(self.handle, half_edge_id)
            },
            _ => panic!("Only subdivision geometries can have half edges!"),
        }
    }

    /// Returns the previous half edge of some half edge.
    pub fn get_previous_half_edge(&self, half_edge_id: u32) -> u32 {
        match self.kind {
            GeometryKind::SUBDIVISION => unsafe {
                rtcGetGeometryPreviousHalfEdge(self.handle, half_edge_id)
            },
            _ => panic!("Only subdivision geometries can have half edges!"),
        }
    }

    /// Returns the opposite half edge of some half edge.
    pub fn get_opposite_half_edge(&self, topology_id: u32, edge_id: u32) -> u32 {
        match self.kind {
            GeometryKind::SUBDIVISION => unsafe {
                rtcGetGeometryOppositeHalfEdge(self.handle, topology_id, edge_id)
            },
            _ => panic!("Only subdivision geometries can have half edges!"),
        }
    }

    /// Sets the displacement function for a subdivision geometry.
    ///
    /// Only one displacement function can be set per geometry, further calls to
    /// this will overwrite the previous displacement function.
    /// Passing `None` will remove the displacement function.
    ///
    /// The registered function is invoked to displace points on the subdivision
    /// geometry during spatial acceleration structure construction,
    /// during the [`Scene::commit`] call.
    ///
    /// The displacement function is called for each vertex of the subdivision
    /// geometry. The function is called with the following parameters:
    ///
    /// * `geometry`: The geometry handle.
    /// * `geometry_user_data`: The user data.
    /// * `prim_id`: The ID of the primitive that contains the vertices to
    ///   displace.
    /// * `time_step`: The time step for which the displacement function is
    ///   evaluated. Important for time dependent displacement and motion blur.
    /// * `vertices`: The information about the vertices to displace. See
    ///   [`Vertices`].
    ///
    /// # Safety
    ///
    /// The callback function provided to this function contains a raw pointer
    /// to Embree geometry.
    pub unsafe fn set_displacement_function<F, D>(&mut self, displacement: F)
    where
        D: UserGeometryData,
        F: for<'a> FnMut(RTCGeometry, Option<&mut D>, u32, u32, Vertices<'a>),
    {
        match self.kind {
            GeometryKind::SUBDIVISION => {
                let mut geom_data = self.data.lock().unwrap();
                unsafe {
                    let mut closure = displacement;
                    geom_data
                        .subdivision_fns
                        .replace(SubdivisionGeometryPayloads {
                            displacement_fn: &mut closure as *mut _ as *mut std::os::raw::c_void,
                        });
                    rtcSetGeometryDisplacementFunction(
                        self.handle,
                        displacement_function(&mut closure),
                    )
                }
            }
            _ => panic!("Only subdivision geometries can have displacement functions!"),
        }
    }

    /// Removes the displacement function for a subdivision geometry.
    pub fn unset_displacement_function(&mut self) {
        match self.kind {
            GeometryKind::SUBDIVISION => unsafe {
                rtcSetGeometryDisplacementFunction(self.handle, None);
            },
            _ => panic!("Only subdivision geometries can have displacement functions!"),
        }
    }

    /// Sets the instanced scene of an instance geometry.
    pub fn set_instanced_scene(&mut self, scene: &Scene) {
        match self.kind {
            GeometryKind::INSTANCE => unsafe {
                rtcSetGeometryInstancedScene(self.handle, scene.handle)
            },
            _ => panic!("Only instance geometries can have instanced scenes!"),
        }
    }

    /// Returns the interpolated instance transformation for the specified time
    /// step.
    ///
    /// The transformation is returned as a 4x4 column-major matrix.
    pub fn get_transform(&mut self, time: f32) -> [f32; 16] {
        match self.kind {
            GeometryKind::INSTANCE => unsafe {
                let mut transform = [0.0; 16];
                rtcGetGeometryTransform(
                    self.handle,
                    time,
                    Format::FLOAT4X4_COLUMN_MAJOR,
                    transform.as_mut_ptr() as *mut _,
                );
                transform
            },
            _ => panic!("Only instance geometries can have instanced scenes!"),
        }
    }

    /// Sets the transformation for a particular time step of an instance
    /// geometry.
    ///
    /// The transformation is specified as a 4x4 column-major matrix.
    pub fn set_transform(&mut self, time_step: u32, transform: &[f32; 16]) {
        match self.kind {
            GeometryKind::INSTANCE => unsafe {
                rtcSetGeometryTransform(
                    self.handle,
                    time_step,
                    Format::FLOAT4X4_COLUMN_MAJOR,
                    transform.as_ptr() as *const _,
                );
            },
            _ => panic!("Only instance geometries can have instanced scenes!"),
        }
    }

    /// Sets the transformation for a particular time step of an instance
    /// geometry as a decomposition of the transformation matrix using
    /// quaternions to represent the rotation.
    pub fn set_transform_quaternion(
        &mut self,
        time_step: u32,
        transform: &QuaternionDecomposition,
    ) {
        match self.kind {
            GeometryKind::INSTANCE => unsafe {
                rtcSetGeometryTransformQuaternion(
                    self.handle,
                    time_step,
                    transform as &QuaternionDecomposition as *const _,
                );
            },
            _ => panic!("Only instance geometries can have instanced scenes!"),
        }
    }
}

/// The arguments for the `Geometry::interpolate` function.
pub struct InterpolateInput {
    pub prim_id: u32,
    pub u: f32,
    pub v: f32,
    pub usage: BufferUsage,
    pub slot: u32,
}

/// The arguments for the `Geometry::interpolate_n` function.
pub struct InterpolateNInput<'a> {
    pub valid: Option<Cow<'a, [u32]>>,
    pub prim_id: Cow<'a, [u32]>,
    pub u: Cow<'a, [f32]>,
    pub v: Cow<'a, [f32]>,
    pub usage: BufferUsage,
    pub slot: u32,
    pub n: u32,
}

/// The output of the `Geometry::interpolate` and `Geometry::interpolate_n`
/// functions in structure of array (SOA) layout.
pub struct InterpolateOutput {
    /// The buffer containing the interpolated values.
    buffer: Vec<f32>,
    /// The number of values per attribute.
    count_per_attribute: u32,
    /// The offset of the `p` attribute in the buffer.
    p_offset: Option<u32>,
    /// The offset of the `dp_du` attribute in the buffer.
    dp_du_offset: Option<u32>,
    /// The offset of the `dp_dv` attribute in the buffer.
    dp_dv_offset: Option<u32>,
    /// The offset of the `ddp_du_du` attribute in the buffer.
    ddp_du_du_offset: Option<u32>,
    /// The offset of the `ddp_dv_dv` attribute in the buffer.
    ddp_dv_dv_offset: Option<u32>,
    /// The offset of the `ddp_du_dv` attribute in the buffer.
    ddp_du_dv_offset: Option<u32>,
}

impl InterpolateOutput {
    pub fn new(count: u32, zeroth_order: bool, first_order: bool, second_order: bool) -> Self {
        assert!(
            count > 0,
            "The number of interpolated values must be greater than 0!"
        );
        assert!(
            zeroth_order || first_order || second_order,
            "At least one of the origin value, first order derivative, or second order derivative \
             must be true!"
        );
        let mut offset = 0;
        let p_offset = zeroth_order.then(|| {
            let _offset = offset;
            offset += count;
            _offset
        });
        let dp_du_offset = first_order.then(|| {
            let _offset = offset;
            offset += count;
            _offset
        });
        let dp_dv_offset = first_order.then(|| {
            let _offset = offset;
            offset += count;
            _offset
        });
        let ddp_du_du_offset = second_order.then(|| {
            let _offset = offset;
            offset += count;
            _offset
        });
        let ddp_dv_dv_offset = second_order.then(|| {
            let _offset = offset;
            offset += count;
            _offset
        });
        let ddp_du_dv_offset = second_order.then(|| {
            let _offset = offset;
            offset += count;
            _offset
        });

        Self {
            buffer: vec![0.0; (offset + count) as usize],
            count_per_attribute: count,
            p_offset,
            dp_du_offset,
            dp_dv_offset,
            ddp_du_du_offset,
            ddp_dv_dv_offset,
            ddp_du_dv_offset,
        }
    }

    /// Returns the interpolated `p` attribute.
    pub fn p(&self) -> Option<&[f32]> {
        self.p_offset.map(|offset| {
            &self.buffer[offset as usize..(offset + self.count_per_attribute) as usize]
        })
    }

    /// Returns the mutable interpolated `p` attribute.
    pub fn p_mut(&mut self) -> Option<&mut [f32]> {
        self.p_offset.map(move |offset| {
            &mut self.buffer[offset as usize..(offset + self.count_per_attribute) as usize]
        })
    }

    /// Returns the interpolated `dp_du` attribute.
    pub fn dp_du(&self) -> Option<&[f32]> {
        self.dp_du_offset.map(|offset| {
            &self.buffer[offset as usize..(offset + self.count_per_attribute) as usize]
        })
    }

    /// Returns the mutable interpolated `dp_du` attribute.
    pub fn dp_du_mut(&mut self) -> Option<&mut [f32]> {
        self.dp_du_offset.map(|offset| {
            &mut self.buffer[offset as usize..(offset + self.count_per_attribute) as usize]
        })
    }

    /// Returns the interpolated `dp_dv` attribute.
    pub fn dp_dv(&self) -> Option<&[f32]> {
        self.dp_dv_offset.map(|offset| {
            &self.buffer[offset as usize..(offset + self.count_per_attribute) as usize]
        })
    }

    /// Returns the mutable interpolated `dp_dv` attribute.
    pub fn dp_dv_mut(&mut self) -> Option<&mut [f32]> {
        self.dp_dv_offset.map(|offset| {
            &mut self.buffer[offset as usize..(offset + self.count_per_attribute) as usize]
        })
    }

    /// Returns the interpolated `ddp_du_du` attribute.
    pub fn ddp_du_du(&self) -> Option<&[f32]> {
        self.ddp_du_du_offset.map(|offset| {
            &self.buffer[offset as usize..(offset + self.count_per_attribute) as usize]
        })
    }

    /// Returns the mutable interpolated `ddp_du_du` attribute.
    pub fn ddp_du_du_mut(&mut self) -> Option<&mut [f32]> {
        self.ddp_du_du_offset.map(|offset| {
            &mut self.buffer[offset as usize..(offset + self.count_per_attribute) as usize]
        })
    }

    /// Returns the interpolated `ddp_dv_dv` attribute.
    pub fn ddp_dv_dv(&self) -> Option<&[f32]> {
        self.ddp_dv_dv_offset.map(|offset| {
            &self.buffer[offset as usize..(offset + self.count_per_attribute) as usize]
        })
    }

    /// Returns the mutable interpolated `ddp_dv_dv` attribute.
    pub fn ddp_dv_dv_mut(&mut self) -> Option<&mut [f32]> {
        self.ddp_dv_dv_offset.map(move |offset| {
            &mut self.buffer[offset as usize..(offset + self.count_per_attribute) as usize]
        })
    }

    /// Returns the interpolated `ddp_du_dv` attribute.
    pub fn ddp_du_dv(&self) -> Option<&[f32]> {
        self.ddp_du_dv_offset.map(|offset| {
            &self.buffer[offset as usize..(offset + self.count_per_attribute) as usize]
        })
    }

    /// Returns the mutable interpolated `ddp_du_dv` attribute.
    pub fn ddp_du_dv_mut(&mut self) -> Option<&mut [f32]> {
        self.ddp_du_dv_offset.map(move |offset| {
            &mut self.buffer[offset as usize..(offset + self.count_per_attribute) as usize]
        })
    }

    /// Returns the number of values per attribute.
    pub fn value_count(&self) -> u32 { self.count_per_attribute }
}

macro_rules! impl_geometry_type {
    ($name:ident, $kind:path, $(#[$meta:meta])*) => {
        #[derive(Debug)]
        pub struct $name<'a>(Geometry<'a>);

        impl<'a> Deref for $name<'a> {
            type Target = Geometry<'a>;

            fn deref(&self) -> &Self::Target { &self.0 }
        }

        impl<'a> DerefMut for $name<'a> {
            fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
        }

        $(#[$meta])*
        impl<'a> $name<'a> {
            pub fn new(device: &Device) -> Result<Self, Error> {
                Ok(Self(Geometry::new(device, $kind)?))
            }
        }
    };
}

impl_geometry_type!(TriangleMesh, GeometryKind::TRIANGLE,
    /// A triangle mesh geometry.
    ///
    /// The index buffer must contain an array of three 32-bit indices per triangle
    /// ([`Format::UINT3`]), and the number of primitives is inferred from the size
    /// of the index buffer.
    ///
    /// The vertex buffer must contain an array of single precision x, y,
    /// and z floating point coordinates per vertex ([`Format::FLOAT3`]), and the
    /// number of vertices is inferred from the size of the vertex buffer.
    /// The vertex buffer can be at most 16 GB in size.
    ///
    /// The parameterization of a triangle uses the first vertex `p0` as the
    /// base point, the vector `p1 - p0` as the u-direction, and the vector
    /// `p2 - p0` as the v-direction. Thus vertex attributes t0, t1, and t2
    /// can be linearly interpolated over the triangle using the barycentric
    /// coordinates `(u,v)` of the hit point:
    ///
    /// t_uv = (1-u-v) * t0 + u * t1 + v * t2
    ///      = t0 + u * (t1 - t0) + v * (t2 - t0)
    ///
    /// A triangle whose vertices are laid out counter-clockwise has its geometry
    /// normal pointing upwards outside the front face.
    ///
    /// For multi-segment motion blur, the number of time steps must be first
    /// specified using the [`Geometry::set_time_step_count`] call. Then a vertex
    /// buffer for each time step can be set using different buffer slots, and all
    /// these buffers have to have the same stride and size.
);

impl_geometry_type!(QuadMesh, GeometryKind::QUAD,
    /// A quad mesh geometry.
    ///
    /// The index buffer must contain an array of four 32-bit indices per triangle
    /// ([`Format::UINT4`]), and the number of primitives is inferred from the size
    /// of the index buffer.
    ///
    /// The vertex buffer must contain an array of single precision x, y,
    /// and z floating point coordinates per vertex ([`Format::FLOAT3`]), and the
    /// number of vertices is inferred from the size of the vertex buffer.
    /// The vertex buffer can be at most 16 GB in size.
    ///
    /// A quad is internally handled as a pair of two triangles `v0`, `v1`, `v3`
    /// and `v2`, `v3`, `v1`, with the `u'/v'` coordinates of the second triangle
    /// corrected by `u = 1-u'` and `v = 1-v'` to produce a quad parametrization
    /// where `u` and `v` are in the range 0 to 1. Thus the parametrization of a quad
    /// uses the first vertex `p0` as base point, and the vector `p1 - p0` as
    /// u-direction, and `p3 - p0` as v-direction. Thus vertex attributes t0, t1, t2, t3
    /// can be bilinearly interpolated over the quadrilateral the following way:
    ///
    /// t_uv = (1-v)((1-u) * t0 + u * t1) + v * ((1-u) * t3 + u * t2)
    ///
    /// Mixed triangle/quad meshes are supported by encoding a triangle as a quad,
    /// which can be achieved by replicating the last triangle vertex (v0,v1,v2 ->
    /// v0,v1,v2,v2). This way the second triangle is a line (which can never get
    /// hit), and the parametrization of the first triangle is compatible with the
    /// standard triangle parametrization.
    /// A quad whose vertices are laid out counter-clockwise has its geometry
    /// normal pointing upwards outside the front face.
    ///
    ///    p3 ------- p2
    ///    ^          |
    ///  v |          |
    ///    |          |
    ///    p0 ------> p1
    ///        u
);

impl_geometry_type!(UserGeometry, GeometryKind::USER,
    /// A user geometry.
);

impl_geometry_type!(Instance, GeometryKind::INSTANCE,
    /// An instance geometry.
);

/// Helper function to convert a Rust closure to `RTCFilterFunctionN` callback
/// for intersect.
fn intersect_filter_function<F, D>(_f: &mut F) -> RTCFilterFunctionN
where
    D: UserGeometryData,
    F: for<'a> FnMut(&'a mut [i32], Option<&mut D>, &mut IntersectContext, RayN<'a>, HitN<'a>),
{
    unsafe extern "C" fn inner<F, D>(args: *const RTCFilterFunctionNArguments)
    where
        D: UserGeometryData,
        F: for<'a> FnMut(&'a mut [i32], Option<&mut D>, &mut IntersectContext, RayN<'a>, HitN<'a>),
    {
        let cb_ptr =
            (*((*args).geometryUserPtr as *mut GeometryData)).intersect_filter_fn as *mut F;
        if !cb_ptr.is_null() {
            let cb = &mut *cb_ptr;
            let user_data = {
                match (*((*args).geometryUserPtr as *mut GeometryData)).user_data {
                    Some(ref user_data) => {
                        if user_data.data.is_null() || user_data.type_id != TypeId::of::<D>() {
                            None
                        } else {
                            Some(&mut *(user_data.data as *mut D))
                        }
                    }
                    None => None,
                }
            };
            cb(
                std::slice::from_raw_parts_mut((*args).valid, (*args).N as usize),
                user_data,
                &mut *(*args).context,
                RayN {
                    ptr: &mut *(*args).ray,
                    len: (*args).N as usize,
                    marker: PhantomData,
                },
                HitN {
                    ptr: &mut *(*args).hit,
                    len: (*args).N as usize,
                    marker: PhantomData,
                },
            );
        }
    }
    Some(inner::<F, D>)
}

/// Helper function to convert a Rust closure to `RTCFilterFunctionN` callback
/// for occluded.
fn occluded_filter_function<F, D>(_f: &mut F) -> RTCFilterFunctionN
where
    D: UserGeometryData,
    F: FnMut(&mut [i32], Option<&mut D>, &mut IntersectContext, RayN, HitN),
{
    unsafe extern "C" fn inner<F, D>(args: *const RTCFilterFunctionNArguments)
    where
        D: UserGeometryData,
        F: FnMut(&mut [i32], Option<&mut D>, &mut IntersectContext, RayN, HitN),
    {
        let len = (*args).N as usize;
        let cb_ptr = (*((*args).geometryUserPtr as *mut GeometryData)).occluded_filter_fn as *mut F;
        if !cb_ptr.is_null() {
            let cb = &mut *cb_ptr;
            let user_data = {
                match (*((*args).geometryUserPtr as *mut GeometryData)).user_data {
                    Some(ref user_data) => {
                        if user_data.data.is_null() || user_data.type_id != TypeId::of::<D>() {
                            None
                        } else {
                            Some(&mut *(user_data.data as *mut D))
                        }
                    }
                    None => None,
                }
            };
            cb(
                std::slice::from_raw_parts_mut((*args).valid, len),
                user_data,
                &mut *(*args).context,
                RayN {
                    ptr: &mut *(*args).ray,
                    len,
                    marker: PhantomData,
                },
                HitN {
                    ptr: &mut *(*args).hit,
                    len,
                    marker: PhantomData,
                },
            );
        }
    }

    Some(inner::<F, D>)
}

/// Helper function to convert a Rust closure to `RTCBoundsFunction` callback.
fn bounds_function<F, D>(_f: &mut F) -> RTCBoundsFunction
where
    D: UserGeometryData,
    F: FnMut(Option<&mut D>, u32, u32, &mut Bounds),
{
    unsafe extern "C" fn inner<F, D>(args: *const RTCBoundsFunctionArguments)
    where
        D: UserGeometryData,
        F: FnMut(Option<&mut D>, u32, u32, &mut Bounds),
    {
        let cb_ptr = (*((*args).geometryUserPtr as *mut GeometryData))
            .user_fns
            .as_ref()
            .expect(
                "User payloads not set! Make sure the geometry was created with kind \
                 GeometryKind::USER",
            )
            .bounds_fn as *mut F;
        if !cb_ptr.is_null() {
            let cb = &mut *cb_ptr;
            let user_data = {
                match (*((*args).geometryUserPtr as *mut GeometryData)).user_data {
                    Some(ref user_data) => {
                        if user_data.data.is_null() || user_data.type_id != TypeId::of::<D>() {
                            None
                        } else {
                            Some(&mut *(user_data.data as *mut D))
                        }
                    }
                    None => None,
                }
            };
            cb(
                user_data,
                (*args).primID,
                (*args).timeStep,
                &mut *(*args).bounds_o,
            );
        }
    }

    Some(inner::<F, D>)
}

/// Helper function to convert a Rust closure to `RTCIntersectFunctionN`
/// callback.
fn intersect_function<F, D>(_f: &mut F) -> RTCIntersectFunctionN
where
    D: UserGeometryData,
    F: for<'a> FnMut(&'a mut [i32], Option<&mut D>, u32, u32, &mut IntersectContext, RayHitN<'a>),
{
    unsafe extern "C" fn inner<F, D>(args: *const RTCIntersectFunctionNArguments)
    where
        D: UserGeometryData,
        F: for<'a> FnMut(
            &'a mut [i32],
            Option<&mut D>,
            u32,
            u32,
            &mut IntersectContext,
            RayHitN<'a>,
        ),
    {
        let cb_ptr = (*((*args).geometryUserPtr as *mut GeometryData))
            .user_fns
            .as_ref()
            .expect(
                "User payloads not set! Make sure the geometry was created with kind \
                 GeometryKind::USER",
            )
            .intersect_fn as *mut F;
        if !cb_ptr.is_null() {
            let cb = &mut *cb_ptr;
            let user_data = {
                match (*((*args).geometryUserPtr as *mut GeometryData)).user_data {
                    Some(ref user_data) => {
                        if user_data.data.is_null() || user_data.type_id != TypeId::of::<D>() {
                            None
                        } else {
                            Some(&mut *(user_data.data as *mut D))
                        }
                    }
                    None => None,
                }
            };
            cb(
                std::slice::from_raw_parts_mut((*args).valid, (*args).N as usize),
                user_data,
                (*args).geomID,
                (*args).primID,
                &mut *(*args).context,
                RayHitN {
                    ptr: (*args).rayhit,
                    len: (*args).N as usize,
                    marker: PhantomData,
                },
            );
        }
    }

    Some(inner::<F, D>)
}

/// Helper function to convert a Rust closure to `RTCOccludedFunctionN`
/// callback.
fn occluded_function<F, D>(_f: &mut F) -> RTCOccludedFunctionN
where
    D: UserGeometryData,
    F: for<'a> FnMut(&'a mut [i32], Option<&mut D>, u32, u32, &mut IntersectContext, RayN<'a>),
{
    unsafe extern "C" fn inner<F, D>(args: *const RTCOccludedFunctionNArguments)
    where
        D: UserGeometryData,
        F: for<'a> FnMut(&'a mut [i32], Option<&mut D>, u32, u32, &mut IntersectContext, RayN<'a>),
    {
        let cb_ptr = (*((*args).geometryUserPtr as *mut GeometryData))
            .user_fns
            .as_ref()
            .expect(
                "User payloads not set! Make sure the geometry was created with kind \
                 GeometryKind::USER",
            )
            .occluded_fn as *mut F;
        if !cb_ptr.is_null() {
            let cb = &mut *cb_ptr;
            let user_data = {
                match (*((*args).geometryUserPtr as *mut GeometryData)).user_data {
                    Some(ref user_data) => {
                        if user_data.data.is_null() || user_data.type_id != TypeId::of::<D>() {
                            None
                        } else {
                            Some(&mut *(user_data.data as *mut D))
                        }
                    }
                    None => None,
                }
            };
            cb(
                std::slice::from_raw_parts_mut((*args).valid, (*args).N as usize),
                user_data,
                (*args).geomID,
                (*args).primID,
                &mut *(*args).context,
                RayN {
                    ptr: (*args).ray,
                    len: (*args).N as usize,
                    marker: PhantomData,
                },
            )
        }
    }

    Some(inner::<F, D>)
}

/// Struct holding data for a set of vertices in SoA layout.
/// This is used as a parameter to the callback function set by
/// [`Geometry::set_displacement_function`].
pub struct Vertices<'a> {
    /// The number of vertices.
    len: usize,
    /// The u coordinates of points to displace.
    u: *const f32,
    /// The v coordinates of points to displace.
    v: *const f32,
    /// The x components of normal of vertices to displace (normalized).
    ng_x: *const f32,
    ///The y component of normal of vertices to displace (normalized).
    ng_y: *const f32,
    /// The z component of normal of vertices to displace (normalized).
    ng_z: *const f32,
    /// The x components of points to displace.
    p_x: *mut f32,
    /// The y components of points to displace.
    p_y: *mut f32,
    /// The z components of points to displace.
    p_z: *mut f32,
    /// To make sure we don't outlive the lifetime of the pointers.
    marker: PhantomData<&'a mut f32>,
}

impl<'a> Vertices<'a> {
    pub fn into_iter_mut(self) -> VerticesIterMut<'a> {
        VerticesIterMut {
            inner: self,
            cur: 0,
        }
    }
}

pub struct VerticesIterMut<'a> {
    inner: Vertices<'a>,
    cur: usize,
}

impl<'a> Iterator for VerticesIterMut<'a> {
    type Item = ([f32; 2], [f32; 3], [&'a mut f32; 3]);

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur < self.inner.len {
            unsafe {
                let u = *self.inner.u.add(self.cur);
                let v = *self.inner.v.add(self.cur);
                let ng_x = *self.inner.ng_x.add(self.cur);
                let ng_y = *self.inner.ng_y.add(self.cur);
                let ng_z = *self.inner.ng_z.add(self.cur);
                let p_x = self.inner.p_x.add(self.cur);
                let p_y = self.inner.p_y.add(self.cur);
                let p_z = self.inner.p_z.add(self.cur);
                self.cur += 1;
                Some((
                    [u, v],
                    [ng_x, ng_y, ng_z],
                    [&mut *p_x, &mut *p_y, &mut *p_z],
                ))
            }
        } else {
            None
        }
    }
}

impl<'a> ExactSizeIterator for VerticesIterMut<'a> {
    fn len(&self) -> usize { self.inner.len - self.cur }
}

/// Helper function to convert a Rust closure to `RTCDisplacementFunctionN`
/// callback.
fn displacement_function<F, D>(_f: &mut F) -> RTCDisplacementFunctionN
where
    D: UserGeometryData,
    F: for<'a> FnMut(RTCGeometry, Option<&mut D>, u32, u32, Vertices<'a>),
{
    unsafe extern "C" fn inner<F, D>(args: *const RTCDisplacementFunctionNArguments)
    where
        D: UserGeometryData,
        F: FnMut(RTCGeometry, Option<&mut D>, u32, u32, Vertices),
    {
        let cb_ptr = (*((*args).geometryUserPtr as *mut GeometryData))
            .subdivision_fns
            .as_ref()
            .expect(
                "User payloads not set! Make sure the geometry was created with kind \
                 GeometryKind::SUBDIVISION",
            )
            .displacement_fn as *mut F;
        if !cb_ptr.is_null() {
            let cb = &mut *cb_ptr;
            let user_data = {
                match (*((*args).geometryUserPtr as *mut GeometryData)).user_data {
                    Some(ref user_data) => {
                        if user_data.data.is_null() || user_data.type_id != TypeId::of::<D>() {
                            None
                        } else {
                            Some(&mut *(user_data.data as *mut D))
                        }
                    }
                    None => None,
                }
            };
            let len = (*args).N as usize;
            let vertices = Vertices {
                len,
                u: (*args).u,
                v: (*args).v,
                ng_x: (*args).Ng_x,
                ng_y: (*args).Ng_y,
                ng_z: (*args).Ng_z,
                p_x: (*args).P_x,
                p_y: (*args).P_y,
                p_z: (*args).P_z,
                marker: PhantomData,
            };
            cb(
                (*args).geometry,
                user_data,
                (*args).primID,
                (*args).timeStep,
                vertices,
            );
        }
    }

    Some(inner::<F, D>)
}
