use std::{borrow::Cow, collections::HashMap, num::NonZeroUsize};

use crate::{sys::*, BufferSlice, BufferUsage, Device, Error, Format, GeometryType};

mod quad_mesh;
mod triangle_mesh;

pub use quad_mesh::QuadMesh;
pub use triangle_mesh::TriangleMesh;

pub trait Geometry {
    fn new(device: &Device) -> Result<Self, Error>
    where
        Self: Sized;
    fn kind(&self) -> GeometryType;
    fn handle(&self) -> RTCGeometry;
    fn commit(&mut self) {
        unsafe {
            rtcCommitGeometry(self.handle());
        }
    }
}

/// A trait for geometry objects that can have vertex attributes.
///
/// Only supported by triangle meshes, quad meshes, curves, points, and
/// subdivision geometries.
pub trait GeometryVertexAttribute: Geometry {
    /// Sets the number of vertex attributes of the geometry.
    ///
    /// This function sets the number of slots for vertex attributes buffers
    /// (BufferUsage::VERTEX_ATTRIBUTE) that can be used for the specified
    /// geometry.
    ///
    /// # Arguments
    ///
    /// * `count` - The number of vertex attribute slots.
    fn set_vertex_attribute_count(&self, count: u32) {
        unsafe {
            rtcSetGeometryVertexAttributeCount(self.handle(), count);
        }
    }
}

#[derive(Debug)]
pub(crate) struct AttachedBuffer<'a: 'static> {
    slot: u32,
    format: Format,
    stride: usize,
    source: BufferSource<'a>,
}

#[derive(Debug)]
enum BufferSource<'a> {
    EmbreeManaged(BufferSlice),
    UserManaged(Cow<'a, [u8]>)
}

/// Handle to an Embree geometry object.
///
/// BufferGeometry is a wrapper around an Embree geometry object. It does not
/// own the buffers that are bound to it, but it does own the geometry object
/// itself.
#[derive(Debug)]
pub struct BufferGeometry<'buf: 'static> {
    pub(crate) device: Device,
    pub(crate) handle: RTCGeometry,
    pub(crate) kind: GeometryType,
    pub(crate) attachments: HashMap<BufferUsage, Vec<AttachedBuffer<'buf>>>,
}

impl<'buf: 'static> Drop for BufferGeometry<'buf> {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseGeometry(self.handle);
        }
    }
}

impl<'device, 'buf: 'static> BufferGeometry<'buf> {
    pub(crate) fn new(device: &'device Device, kind: GeometryType) -> Result<BufferGeometry<'static>, Error> {
        let handle = unsafe { rtcNewGeometry(device.handle, kind) };
        if handle.is_null() {
            Err(device.get_error())
        } else {
            Ok(BufferGeometry {
                device: device.clone(),
                handle,
                kind,
                attachments: HashMap::new(),
            })
        }
    }

    /// Binds a view of a buffer to the geometry.
    ///
    /// After successful completion of this function, the geometry will hold
    /// a reference to the buffer object.
    ///
    /// Analogous to [`rtcSetGeometryBuffer`](https://spec.oneapi.io/oneart/0.5-rev-1/embree-spec.html#rtcsetgeometrybuffer).
    ///
    /// # Arguments
    ///
    /// * `usage` - The usage of the buffer.
    ///
    /// * `slot` - The slot to bind the buffer to.
    ///
    /// * `format` - The format of the buffer.
    ///
    /// * `slice` - The buffer slice to bind.
    ///
    /// * `stride` - The stride of the elements in the buffer. Must be a
    ///   multiple of 4.
    ///
    /// * `count` - The number of elements in the buffer.
    pub fn set_buffer(
        &mut self,
        usage: BufferUsage,
        slot: u32,
        format: Format,
        slice: BufferSlice,
        stride: usize,
        count: usize,
    ) -> Result<(), Error> {
        debug_assert!(stride % 4 == 0, "Stride must be a multiple of 4!");
        match slice {
            BufferSlice::External {
                buffer,
                offset,
                size,
            } => {
                let bindings = self.attachments.entry(usage).or_insert_with(Vec::new);
                println!(
                    "Binding buffer to slot {}, offset {}, stride {}, count {}",
                    slot, offset, stride, count
                );
                match bindings.iter().position(|a| a.slot == slot) {
                    Some(i) => {
                        eprint!(
                            "Buffer already attached to slot {}, will be overwritten!",
                            slot
                        );
                        bindings.remove(i);
                        unsafe {
                            rtcSetGeometryBuffer(
                                self.handle,
                                usage,
                                slot,
                                format,
                                buffer.handle,
                                offset,
                                stride as usize,
                                count as usize,
                            )
                        };
                        bindings.push(AttachedBuffer {
                            slot,
                            source: BufferSource::EmbreeManaged(BufferSlice::External {
                                buffer,
                                offset,
                                size,
                            }),
                            format,
                            stride,
                        });
                        Ok(())
                    }
                    None => {
                        unsafe {
                            rtcSetGeometryBuffer(
                                self.handle,
                                usage,
                                slot,
                                format,
                                buffer.handle,
                                offset,
                                stride as usize,
                                count as usize,
                            )
                        };
                        bindings.push(AttachedBuffer {
                            slot,
                            source: BufferSource::EmbreeManaged(BufferSlice::External {
                                buffer,
                                offset,
                                size,
                            }),
                            format,
                            stride,
                        });
                        Ok(())
                    }
                }
            }
            BufferSlice::Internal { .. } => {
                eprint!("Internally managed buffer cannot be shared!");
                Err(Error::INVALID_ARGUMENT)
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
    ) -> Result<(), Error> {
        debug_assert!(stride % 4 == 0, "Stride must be a multiple of 4!");
        let bindings = self.attachments.entry(usage).or_insert_with(Vec::new);
        if bindings.iter().find(|a| a.slot == slot).is_none() {
            let raw_ptr = unsafe {
                rtcSetNewGeometryBuffer(
                    self.handle,
                    usage,
                    slot,
                    format,
                    stride as usize,
                    count as usize,
                )
            };
            if raw_ptr.is_null() {
                Err(self.device.get_error())
            } else {
                bindings.push(AttachedBuffer {
                    slot,
                    source: BufferSource::EmbreeManaged(BufferSlice::Internal {
                        ptr: raw_ptr,
                        size: NonZeroUsize::new(count * stride as usize).unwrap(),
                        marker: std::marker::PhantomData,
                    }),
                    format,
                    stride,
                });
                Ok(())
            }
        } else {
            eprint!("Buffer already attached to slot {}", slot);
            Err(Error::INVALID_ARGUMENT)
        }
    }

    /// Returns the buffer bound to the given slot and usage.
    pub fn get_buffer(&self, usage: BufferUsage, slot: u32) -> Option<&BufferSlice> {
        let attachment = self.attachments
            .get(&usage)
            .and_then(|v| v.iter().find(|a| a.slot == slot));
        if let Some(attachment) = attachment {
            match attachment.source {
                BufferSource::EmbreeManaged(ref slice) => Some(slice),
                BufferSource::UserManaged(_) => None,
            }
        } else {
            None
        }
    }

    pub fn commit(&mut self) {
        unsafe {
            rtcCommitGeometry(self.handle);
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

    // /// Assigns a view of a shared data buffer to a geometry.
    // pub fn set_shared_buffer(&mut self, usage: BufferUsage, slot: u32, buffer:
    // &[u8], ) -> Result<(), Error> {     let bindings =
    // self.attachments.entry(usage).or_insert_with(Vec::new);     if bindings.
    // iter().find(|a| a.slot == slot).is_none() {         bindings.
    // push(AttachedBuffer {             slot,
    //             buffer,
    //             format: Format::FLOAT3,
    //             stride: 0,
    //         });
    //         Ok(())
    //     } else {
    //         eprint!("Buffer already attached to slot {}", slot);
    //         Err(Error::INVALID_ARGUMENT)
    //     }
    // }
}
