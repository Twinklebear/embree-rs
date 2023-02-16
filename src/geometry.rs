use std::{
    collections::HashMap,
    marker::PhantomData,
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

use crate::{sys::*, BufferSlice, BufferUsage, Device, Error, Format, GeometryType};

// mod quad_mesh;
// mod triangle_mesh;

// pub use quad_mesh::QuadMesh;
// pub use triangle_mesh::TriangleMesh;

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

/// State of a geometry object.
#[derive(Default, Debug, Clone)]
struct GeometryState<'buf> {
    vertex_attribute_count: Option<u32>,
    attachments: HashMap<BufferUsage, Vec<AttachedBuffer<'buf>>>,
}

/// Wrapper around an Embree geometry object.
///
/// It does not own the buffers that are bound to it, but it does own the
/// geometry object itself.
#[derive(Debug)]
pub struct Geometry<'buf> {
    pub(crate) device: Device,
    pub(crate) handle: RTCGeometry,
    kind: GeometryType,
    state: Arc<Mutex<GeometryState<'buf>>>,
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

impl<'dev, 'buf> Geometry<'buf> {
    pub(crate) fn new(device: &'dev Device, kind: GeometryType) -> Result<Geometry<'buf>, Error> {
        let handle = unsafe { rtcNewGeometry(device.handle, kind) };
        let vertex_attribute_count = match kind {
            GeometryType::GRID | GeometryType::USER | GeometryType::INSTANCE => None,
            _ => Some(0),
        };
        if handle.is_null() {
            Err(device.get_error())
        } else {
            Ok(Geometry {
                device: device.clone(),
                handle,
                kind,
                state: Arc::new(Mutex::new(GeometryState {
                    vertex_attribute_count,
                    attachments: HashMap::new(),
                })),
            })
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
        match self.state.lock().unwrap().vertex_attribute_count {
            None => {
                eprint!(
                    "Vertex attribute not allowed for geometries of type {:?}!",
                    self.kind
                );
                Err(Error::INVALID_OPERATION)
            }
            Some(c) => {
                if slot >= c {
                    eprint!(
                        "Vertex attribute slot {} is out of bounds for geometry of type {:?}!",
                        slot, self.kind
                    );
                    Err(Error::INVALID_ARGUMENT)
                } else {
                    Ok(())
                }
            }
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
    ) -> Result<BufferSlice, Error> {
        debug_assert!(stride % 4 == 0, "Stride must be a multiple of 4!");
        if usage == BufferUsage::VERTEX_ATTRIBUTE {
            self.check_vertex_attribute(slot)?;
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
        let state = self.state.lock().unwrap();
        state
            .attachments
            .get(&usage)
            .and_then(|v| v.iter().find(|a| a.slot == slot))
            .map(|a| a.source)
    }

    /// Returns the type of geometry of this geometry.
    pub fn kind(&self) -> GeometryType { self.kind }

    pub fn commit(&mut self) {
        unsafe {
            rtcCommitGeometry(self.handle);
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
        let mut state = self.state.lock().unwrap();
        match state.vertex_attribute_count {
            None => {
                panic!(
                    "set_vertex_attribute_count is not supported by geometry of type {:?}.",
                    self.kind
                );
            }
            Some(_) => {
                // Update the vertex attribute count.
                unsafe {
                    rtcSetGeometryVertexAttributeCount(self.handle, count);
                }
                state.vertex_attribute_count = Some(count);
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
    ($name:ident, $kind:path, #[$doc:meta]) => {
        #[derive(Debug)]
        pub struct $name(Geometry<'static>);

        impl Deref for $name {
            type Target = Geometry<'static>;

            fn deref(&self) -> &Self::Target { &self.0 }
        }

        impl DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
        }

        impl $name {
            #[$doc]
            pub fn new(device: &Device) -> Result<Self, Error> {
                Ok(Self(Geometry::new(device, $kind)?))
            }
        }
    };
}

use std::ops::{Deref, DerefMut};

impl_geometry_type!(TriangleMesh, GeometryType::TRIANGLE, #[doc = "A triangle mesh geometry."]);
impl_geometry_type!(QuadMesh, GeometryType::QUAD, #[doc = "A quad mesh geometry."]);
