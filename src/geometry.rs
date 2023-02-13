use std::collections::HashMap;
use std::num::NonZeroUsize;

use crate::{sys::*, Device, Error};
use crate::{BufferUsage, Format, GeometryType, BufferSlice};

mod triangle_mesh;

pub use triangle_mesh::TriangleMesh;

pub trait Geometry {
    fn new(device: &Device) -> Result<Self, Error> where Self: Sized;
    fn kind(&self) -> GeometryType;
    fn handle(&self) -> RTCGeometry;
    fn commit(&mut self) {
        unsafe {
            rtcCommitGeometry(self.handle());
        }
    }
}

#[derive(Debug)]
pub(crate) struct AttachedBuffer {
    pub slot: u32,
    pub buffer: BufferSlice,
    pub format: Format,
    pub stride: usize,
}

/// Handle to an Embree geometry object.
///
/// BufferGeometry is a wrapper around an Embree geometry object. It does not own the
/// buffers that are bound to it, but it does own the geometry object itself.
#[derive(Debug)]
pub struct BufferGeometry {
    pub(crate) device: Device,
    pub(crate) handle: RTCGeometry,
    pub(crate) kind: GeometryType,
    pub(crate) attachments: HashMap<BufferUsage, Vec<AttachedBuffer>>,
}

impl<'a> Drop for BufferGeometry {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseGeometry(self.handle);
        }
    }
}

impl BufferGeometry {
    pub(crate) fn new(device: &Device, kind: GeometryType) -> Result<BufferGeometry, Error> {
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
    /// * `stride` - The stride of the elements in the buffer.
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
        match slice {
            BufferSlice::Created {
                buffer,
                offset,
                size,
            } => {
                let mut bindings = self.attachments.entry(usage).or_insert_with(Vec::new);
                if bindings.iter().find(|a| a.slot == slot).is_none() {
                    println!("Binding buffer to slot {}, offset {}, stride {}, count {}", slot,
                    offset, stride, count);
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
                        buffer: BufferSlice::Created {
                            buffer,
                            offset,
                            size,
                        },
                        format,
                        stride,
                    });
                    Ok(())
                } else {
                    eprint!("Buffer already attached to slot {}", slot);
                    Err(Error::INVALID_ARGUMENT)
                }
            }
            BufferSlice::Managed { .. } => {
                eprint!("Internal buffer cannot be shared!");
                Err(Error::INVALID_ARGUMENT)
            }
        }
    }

    /// Creates a new [`Buffer`] and binds it as a specific attribute for this geometry.
    ///
    /// Analogous to [`rtcSetNewGeometryBuffer`](https://spec.oneapi.io/oneart/0.5-rev-1/embree-spec.html#rtcsetnewgeometrybuffer).
    ///
    /// The allocated buffer will be automatically over-allocated slightly when used as a
    /// [`BufferUsage::VERTEX`] buffer, where a requirement is that each buffer element should
    /// be readable using 16-byte SSE load instructions.
    ///
    /// The allocated buffer is managed internally and automatically released when the geometry
    /// is destroyed by Embree.
    ///
    /// # Arguments
    ///
    /// * `usage` - The usage of the buffer.
    ///
    /// * `slot` - The slot to bind the buffer to.
    ///
    /// * `format` - The format of the buffer items. See [`Format`] for more information.
    ///
    /// * `count` - The number of items in the buffer.
    ///
    /// * `stride` - The stride of the buffer items. MUST be aligned to 4 bytes.
    pub fn set_new_buffer(
        &mut self,
        usage: BufferUsage,
        slot: u32,
        format: Format,
        stride: usize,
        count: usize,
    ) -> Result<(), Error> {
        let bindings = self.attachments.entry(usage).or_insert_with(Vec::new);
        if bindings.iter().find(|a| a.slot == slot).is_none() {
            let raw_ptr = unsafe {
                rtcSetNewGeometryBuffer(self.handle, usage, slot, format, stride as usize, count as usize)
            };
            if raw_ptr.is_null() {
                Err(self.device.get_error())
            } else {
                let buffer_slice = BufferSlice::Managed {
                    ptr: raw_ptr,
                    size: NonZeroUsize::new(count * stride as usize).unwrap(),
                    marker: std::marker::PhantomData,
                };
                bindings.push(AttachedBuffer {
                    slot,
                    buffer: buffer_slice,
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
        self.attachments
            .get(&usage)
            .and_then(|v| v.iter().find(|a| a.slot == slot))
            .map(|a| &a.buffer)
    }

    pub fn commit(&mut self) {
        unsafe {
            rtcCommitGeometry(self.handle);
        }
    }

    /// Set the subdivision mode for the topology of the specified subdivision
    /// geometry.
    ///
    /// The subdivision modes can be used to force linear interpolation for certain
    /// parts of the subdivision mesh:
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
    /// interpolated linearly. This mode is typically used for texturing to also map
    /// texels at the border of the texture to the mesh.
    ///
    /// * [`RTCSubdivisionMode::PIN_ALL`]: All vertices at the border are pinned
    /// to their location during subdivision. This way all patches are linearly
    /// interpolated.
    fn set_subdivision_mode(&self, topology_id: u32, mode: RTCSubdivisionMode) {
        unsafe {
            rtcSetGeometrySubdivisionMode(self.handle, topology_id, mode);
        }
    }

    /// Binds a vertex attribute to a topology of the geometry.
    ///
    /// This function binds a vertex attribute buffer slot to a topology for the
    /// specified subdivision geometry. Standard vertex buffers are always bound to
    /// the default topology (topology 0) and cannot be bound differently. A vertex
    /// attribute buffer always uses the topology it is bound to when used in the
    /// `rtcInterpolate` and `rtcInterpolateN` calls.
    ///
    /// A topology with ID `i` consists of a subdivision mode set through
    /// `Geometry::set_subdivision_mode` and the index buffer bound to the index
    /// buffer slot `i`. This index buffer can assign indices for each face of the
    /// subdivision geometry that are different to the indices of the default topology.
    /// These new indices can for example be used to introduce additional borders into
    /// the subdivision mesh to map multiple textures onto one subdivision geometry.
    fn set_vertex_attribute_topology(&self, vertex_attribute_id: u32, topology_id: u32) {
        unsafe {
            rtcSetGeometryVertexAttributeTopology(self.handle, vertex_attribute_id, topology_id);
        }
    }
}
