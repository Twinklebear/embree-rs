//!

use std::{
    collections::HashMap,
    marker::PhantomData,
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

use crate::{sys::*, BufferSlice, BufferUsage, BuildQuality, Device, Error, Format, GeometryType};

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
/// scene by using the [`Scene::attach_geometry`] function (to automatically
/// assign a geometry ID) or using the [`Scene::attach_geometry_by_id`] function
/// (to specify the geometry ID manually). A geometry can get attached to
/// multiple scenes.
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
    kind: GeometryType,
    attachments: Arc<Mutex<HashMap<BufferUsage, Vec<AttachedBuffer<'buf>>>>>,
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

impl<'dev, 'buf> Geometry<'buf> {
    /// Creates a new geometry object.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use embree::{Device, Geometry, GeometryType};
    ///
    /// let device = Device::new().unwrap();
    /// let geometry = Geometry::new(&device, GeometryType::TRIANGLE).unwrap();
    /// ```
    ///
    /// or use the [`Device::create_geometry`] method:
    ///
    /// ```no_run
    /// use embree::{Device, GeometryType};
    ///
    /// let device = Device::new().unwrap();
    /// let geometry = device.create_geometry(GeometryType::TRIANGLE).unwrap();
    /// ```
    pub fn new(device: &'dev Device, kind: GeometryType) -> Result<Geometry<'buf>, Error> {
        let handle = unsafe { rtcNewGeometry(device.handle, kind) };
        if handle.is_null() {
            Err(device.get_error())
        } else {
            Ok(Geometry {
                device: device.clone(),
                handle,
                kind,
                attachments: Arc::new(Mutex::new(HashMap::new())),
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

    /// Checks if the given vertex attribute slot is valid for this geometry.
    fn check_vertex_attribute(&self, slot: u32) -> Result<(), Error> {
        match self.kind {
            GeometryType::GRID | GeometryType::USER | GeometryType::INSTANCE => {
                eprint!(
                    "Vertex attribute not allowed for geometries of type {:?}!",
                    self.kind
                );
                Err(Error::INVALID_OPERATION)
            }
            _ => Ok(())
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
            self.check_vertex_attribute(slot)?;
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
            self.check_vertex_attribute(slot)?;
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
                        marker: std::marker::PhantomData,
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
    pub fn kind(&self) -> GeometryType { self.kind }

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
            GeometryType::GRID | GeometryType::USER | GeometryType::INSTANCE => {
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
    pub fn set_subdivision_mode(&self, topology_id: u32, mode: RTCSubdivisionMode) {
        unsafe {
            rtcSetGeometrySubdivisionMode(self.handle, topology_id, mode);
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

use std::ops::{Deref, DerefMut};

impl_geometry_type!(TriangleMesh, GeometryType::TRIANGLE,
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

impl_geometry_type!(QuadMesh, GeometryType::QUAD,
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
