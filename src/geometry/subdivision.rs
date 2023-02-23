use crate::{
    geometry::GeometryData, sys, Device, Error, Geometry, GeometryKind, SubdivisionMode,
    UserGeometryData,
};
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};
use sys::*;

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
        unsafe { rtcSetGeometrySubdivisionMode(self.handle, topology_id, mode) }
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
        unsafe { rtcSetGeometryTopologyCount(self.handle, count) }
    }

    /// Returns the first half edge of a face.
    ///
    /// This function can only be used for subdivision meshes. As all topologies
    /// of a subdivision geometry share the same face buffer the function does
    /// not depend on the topology ID.
    pub fn get_first_half_edge(&self, face_id: u32) -> u32 {
        unsafe { rtcGetGeometryFirstHalfEdge(self.handle, face_id) }
    }

    /// Returns the face of some half edge.
    ///
    /// This function can only be used for subdivision meshes. As all topologies
    /// of a subdivision geometry share the same face buffer the function does
    /// not depend on the topology ID.
    pub fn get_face(&self, half_edge_id: u32) -> u32 {
        unsafe { rtcGetGeometryFace(self.handle, half_edge_id) }
    }

    /// Returns the next half edge of some half edge.
    ///
    /// This function can only be used for subdivision meshes. As all topologies
    /// of a subdivision geometry share the same face buffer the function does
    /// not depend on the topology ID.
    pub fn get_next_half_edge(&self, half_edge_id: u32) -> u32 {
        unsafe { rtcGetGeometryNextHalfEdge(self.handle, half_edge_id) }
    }

    /// Returns the previous half edge of some half edge.
    pub fn get_previous_half_edge(&self, half_edge_id: u32) -> u32 {
        unsafe { rtcGetGeometryPreviousHalfEdge(self.handle, half_edge_id) }
    }

    /// Returns the opposite half edge of some half edge.
    pub fn get_opposite_half_edge(&self, topology_id: u32, edge_id: u32) -> u32 {
        unsafe { rtcGetGeometryOppositeHalfEdge(self.handle, topology_id, edge_id) }
    }

    // TODO(yang): Better way to deal with RTCGeometry, maybe we need a lookup table
    // to get the geometry from the handle.
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
    /// * `geometry_user_data`: The user data pointer that was specified when
    ///   the geometry was created.
    /// * `geometry`: The geometry handle.
    /// * `prim_id`: The ID of the primitive that contains the vertices to
    ///   displace.
    /// * `time_step`: The time step for which the displacement function is
    ///   evaluated. Important for time dependent displacement and motion blur.
    /// * `us`: The u coordinates of points to displace.
    /// * `vs`: The v coordinates of points to displace.
    /// * `ng_xs`: The x components of normal of vertices to displace
    ///   (normalized).
    /// * `ng_ys`: The y component of normal of vertices to displace
    ///   (normalized).
    /// * `ng_ys`: The z component of normal of vertices to displace
    ///   (normalized).
    /// * `pxs`: The x components of points to displace.
    /// * `pys`: The y components of points to displace.
    /// * `pzs`: The z components of points to displace.
    ///
    /// # Safety
    ///
    /// The callback function provided to this function contains a raw pointer
    /// to Embree geometry.
    pub unsafe fn set_displacement_function<F, D>(&mut self, displacement: F)
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
                displacement_function(&mut closure),
            )
        }
    }

    /// Removes the displacement function for a subdivision geometry.
    pub fn unset_displacement_function(&mut self) {
        unsafe {
            sys::rtcSetGeometryDisplacementFunction(self.handle, None);
        }
    }
}

/// Helper function to convert a Rust closure to `RTCDisplacementFunctionN`
/// callback.
fn displacement_function<F, D>(_f: &mut F) -> RTCDisplacementFunctionN
where
    D: UserGeometryData,
    F: FnMut(
        Option<&mut D>,
        RTCGeometry,
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
    unsafe extern "C" fn inner<F, D>(args: *const RTCDisplacementFunctionNArguments)
    where
        D: UserGeometryData,
        F: FnMut(
            Option<&mut D>,
            RTCGeometry,
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
            cb(
                user_data,
                (*args).geometry,
                (*args).primID,
                (*args).timeStep,
                std::slice::from_raw_parts((*args).u, (*args).N as usize),
                std::slice::from_raw_parts((*args).v, (*args).N as usize),
                std::slice::from_raw_parts((*args).Ng_x, (*args).N as usize * 3),
                std::slice::from_raw_parts((*args).Ng_y, (*args).N as usize * 3),
                std::slice::from_raw_parts((*args).Ng_z, (*args).N as usize * 3),
                std::slice::from_raw_parts_mut((*args).P_x, (*args).N as usize * 3),
                std::slice::from_raw_parts_mut((*args).P_y, (*args).N as usize * 3),
                std::slice::from_raw_parts_mut((*args).P_z, (*args).N as usize * 3),
            );
        }
    }

    Some(inner::<F, D>)
}
