use std::collections::HashMap;

use crate::buffer::BufferSlice;
use crate::{sys::*, Device, Error};
use crate::{BufferUsage, Format, GeometryType};

mod triangle_mesh;

pub trait Geometry {
    fn geometry_type(&self) -> GeometryType;
}

/// Handle to an Embree geometry object.
pub struct GeometryData {
    pub(crate) device: Device,
    pub(crate) handle: RTCGeometry,
    pub(crate) kind: GeometryType,
    pub(crate) bindings: HashMap<BufferUsage, Vec<BufferSlice>>,
}

impl Drop for GeometryData {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseGeometry(self.handle);
        }
    }
}

impl GeometryData {
    pub(crate) fn new(device: Device, kind: GeometryType) -> Result<GeometryData, Error> {
        let handle = unsafe { rtcNewGeometry(device.handle, kind) };
        if handle.is_null() {
            Err(device.get_error())
        } else {
            Ok(GeometryData {
                device,
                handle,
                kind,
                bindings: HashMap::new(),
            })
        }
    }

    // pub fn bind_new_buffer<T: Copy>(
    //     &mut self,
    //     usage: BufferUsage,
    //     format: Format,
    //     data: &[T],
    // ) -> Result<BufferSlice, Error> {
    //     let buffer = self.device.create_buffer(usage, format, data)?;
    //     self.bind_buffer(usage, buffer)
    // }

    /// Binds a view of a buffer to a geometry.
    pub fn bind_buffer<T>(
        &mut self,
        buffer: BufferSlice,
        slot: u32,
        usage: BufferUsage,
        format: Format,
    ) -> Result<(), Error> {
        self.bindings.entry(usage).or_insert(vec![])
        unsafe {
            rtcSetGeometryBuffer(
                self.handle,
                usage,
                slot,
                format,
                buffer,
                buffer.offset,
                std::mem::size_of::<T>(),
                buffer.size / std::mem::size_of::<T>(),
            )
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
            rtcSetGeometryVertexAttributeTopology(self.handle(), vertex_attribute_id, topology_id);
        }
    }
}
