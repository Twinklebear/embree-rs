use std::{
    any::TypeId, collections::HashMap, marker::PhantomData, num::NonZeroUsize, ptr, sync::Mutex,
};

use crate::{sys::*, BufferSlice, BufferUsage, BuildQuality, Device, Error, Format, GeometryKind, HitN, HitPacket, IntersectContext, RayN, RayPacket, Scene};

use std::{
    ops::{Deref, DerefMut},
    rc::Rc,
};

mod instance;
mod subdivision;
mod user_geom;

pub use instance::*;
pub use subdivision::*;
pub use user_geom::*;

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
pub(crate) struct UserData {
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
    pub user_data: Option<UserData>,
    /// Payload for the [`Geometry::set_intersect_filter_function`] call.
    pub intersect_filter_fn: *mut std::os::raw::c_void,
    /// Payload for the [`Geometry::set_occluded_filter_function`] call.
    pub occluded_filter_fn: *mut std::os::raw::c_void,
    /// Payloads only used for user geometry.
    pub user_fns: Option<UserGeometryPayloads>,
    /// Payloads only used for subdivision geometry.
    pub subdivision_fns: Option<SubdivisionGeometryPayloads>,
}

/// Extra data for a geometry.
#[derive(Debug, Clone)]
struct GeometryState<'buf> {
    attachments: HashMap<BufferUsage, Vec<AttachedBuffer<'buf>>>,
    data: GeometryData,
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
    state: Rc<Mutex<GeometryState<'buf>>>,
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
            state: self.state.clone(),
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
            let state = GeometryState {
                attachments: Default::default(),
                data: GeometryData {
                    user_data: None,
                    intersect_filter_fn: ptr::null_mut(),
                    occluded_filter_fn: ptr::null_mut(),
                    user_fns: if kind == GeometryKind::USER {
                        Some(UserGeometryPayloads::default())
                    } else {
                        None
                    },
                    subdivision_fns: if kind == GeometryKind::SUBDIVISION {
                        Some(SubdivisionGeometryPayloads::default())
                    } else {
                        None
                    },
                },
            };
            Ok(Geometry {
                device: device.clone(),
                handle,
                kind,
                state: Rc::new(Mutex::new(state)),
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
                let mut state = self.state.lock().unwrap();
                let bindings = state.attachments.entry(usage).or_insert_with(Vec::new);
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
                let mut state = self.state.lock().unwrap();
                let bindings = state.attachments.entry(usage).or_insert_with(Vec::new);
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
            let mut state = self.state.lock().unwrap();
            let bindings = state.attachments.entry(usage).or_insert_with(Vec::new);
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
        let state = self.state.lock().unwrap();
        state
            .attachments
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
    /// Passing `None` as the filter function removes the filter function.
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
        F: FnMut(&mut [i32], Option<&mut D>, &mut IntersectContext, RayN, HitN),
    {
        let mut state = self.state.lock().unwrap();
        unsafe {
            let mut closure = filter;
            state.data.intersect_filter_fn = &mut closure as *mut _ as *mut std::os::raw::c_void;
            rtcSetGeometryIntersectFilterFunction(
                self.handle,
                crate::callback::intersect_filter_function_helper(&mut closure),
            );
        }
    }

    /// Sets the occlusion filter for the geometry.
    ///
    /// Only a single callback function can be registered per geometry, and
    /// further invocations overwrite the previously set callback function.
    /// Passing `None` as the filter function removes the filter function.
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
        let mut state = self.state.lock().unwrap();
        unsafe {
            let mut closure = filter;
            state.data.occluded_filter_fn = &mut closure as *mut _ as *mut std::os::raw::c_void;
            rtcSetGeometryOccludedFilterFunction(
                self.handle,
                crate::callback::occluded_filter_function_helper(&mut closure),
            );
        }
    }

    /// Sets the point query callback function for a geometry.
    ///
    /// Only a single callback function can be registered per geometry and
    /// further invocations overwrite the previously set callback function.
    /// Passing `None` as function pointer disables the registered callback
    /// function.
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
    pub unsafe fn set_point_query_function(&mut self, query_func: RTCPointQueryFunction) {
        rtcSetGeometryPointQueryFunction(self.handle, query_func);
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
        let mut state = self.state.lock().unwrap();
        state.data.user_data = Some(UserData {
            data: user_data as *mut D as *mut std::os::raw::c_void,
            type_id: TypeId::of::<D>(),
        });
        unsafe {
            rtcSetGeometryUserData(self.handle, &mut state.data as *mut GeometryData as *mut _);
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
                    Some(user_data @ UserData { .. }) => {
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
}

// TODO(yang): rtcInterpolate, rtcInterpolateN

macro_rules! impl_geometry_type {
    ($name:ident, $kind:path, $(#[$meta:meta])*) => {
        #[derive(Debug)]
        pub struct $name(Geometry<'static>);

        impl Deref for $name {
            type Target = Geometry<'static>;

            fn deref(&self) -> &Self::Target { &self.0 }
        }

        impl DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
        }

        $(#[$meta])*
        impl $name {
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
