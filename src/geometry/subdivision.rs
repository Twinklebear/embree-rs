use crate::{
    sys, sys::RTCDisplacementFunctionN, Device, Error, Geometry, GeometryKind, SubdivisionMode,
    UserGeometryData,
};
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct SubdivisionGeometry(Geometry<'static>);

impl Deref for SubdivisionGeometry {
    type Target = Geometry<'static>;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for SubdivisionGeometry {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl SubdivisionGeometry {
    pub fn new(device: &Device) -> Result<Self, Error> {
        Ok(Self(Geometry::new(device, GeometryKind::SUBDIVISION)?))
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
        unsafe { sys::rtcSetGeometrySubdivisionMode(self.handle, topology_id, mode) }
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
        unsafe { sys::rtcSetGeometryTopologyCount(self.handle, count) }
    }

    /// Returns the first half edge of a face.
    ///
    /// This function can only be used for subdivision meshes. As all topologies
    /// of a subdivision geometry share the same face buffer the function does
    /// not depend on the topology ID.
    pub fn get_first_half_edge(&self, face_id: u32) -> u32 {
        unsafe { sys::rtcGetGeometryFirstHalfEdge(self.handle, face_id) }
    }

    /// Returns the face of some half edge.
    ///
    /// This function can only be used for subdivision meshes. As all topologies
    /// of a subdivision geometry share the same face buffer the function does
    /// not depend on the topology ID.
    pub fn get_face(&self, half_edge_id: u32) -> u32 {
        unsafe { sys::rtcGetGeometryFace(self.handle, half_edge_id) }
    }

    /// Returns the next half edge of some half edge.
    ///
    /// This function can only be used for subdivision meshes. As all topologies
    /// of a subdivision geometry share the same face buffer the function does
    /// not depend on the topology ID.
    pub fn get_next_half_edge(&self, half_edge_id: u32) -> u32 {
        unsafe { sys::rtcGetGeometryNextHalfEdge(self.handle, half_edge_id) }
    }

    /// Returns the previous half edge of some half edge.
    pub fn get_previous_half_edge(&self, half_edge_id: u32) -> u32 {
        unsafe { sys::rtcGetGeometryPreviousHalfEdge(self.handle, half_edge_id) }
    }

    /// Returns the opposite half edge of some half edge.
    pub fn get_opposite_half_edge(&self, topology_id: u32, edge_id: u32) -> u32 {
        unsafe { sys::rtcGetGeometryOppositeHalfEdge(self.handle, topology_id, edge_id) }
    }

    // TODO(yang): Add documentation.
    // TODO(yang): Better way to deal with RTCGeometry, maybe we need a lookup table to get the geometry from the handle.
    /// Sets the displacement function for a subdivision geometry.
    pub unsafe fn set_displacement_function<F, D>(&self, displacement: F)
    where
        D: UserGeometryData,
        F: FnMut(
            Option<&mut D>,
            sys::RTCGeometry,
            u32,
            u32,
            &[f32],
            &[f32],
            &[f32],
            &[f32],
            &[f32],
            &mut [f32],
            &mut [f32],
            &mut [f32],
        ),
    {
        let mut state = self.state.lock().unwrap();
        unsafe {
            let mut closure = displacement;
            state.data.intersect_filter_fn = &mut closure as *mut _ as *mut std::os::raw::c_void;
            sys::rtcSetGeometryDisplacementFunction(
                self.handle,
                crate::callback::subdivision_displacement_function_helper(&mut closure),
            )
        }
    }
}
