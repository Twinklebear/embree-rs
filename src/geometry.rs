use std::collections::HashMap;

use crate::{sys::*, Device, Error};
use crate::{Buffer, BufferType, GeometryType};

/// Handle to an Embree geometry object.
pub struct Geometry {
    pub(crate) device: Device,
    pub(crate) handle: RTCGeometry,
    attachments: HashMap<BufferType, Vec<Buffer>>,
}

impl Drop for Geometry {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseGeometry(self.handle);
        }
    }
}

impl Geometry {
    pub(crate) fn new(device: Device, kind: GeometryType) -> Result<Geometry, Error> {
        let handle = unsafe { rtcNewGeometry(device.handle, kind) };
        if handle.is_null() {
            Err(device.get_error())
        } else {
            Ok(Geometry {
                device,
                handle,
                attachments: HashMap::new(),
            })
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
