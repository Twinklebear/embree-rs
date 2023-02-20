use std::ops::{Deref, DerefMut};
use std::ptr;
use crate::{Bounds, Device, Error, Geometry, GeometryKind, IntersectContext, sys, UserData};
use crate::sys::{RTCRayHitN, RTCRayN};

#[derive(Debug)]
pub struct UserGeometry(Geometry<'static>);

impl Deref for UserGeometry {
    type Target = Geometry<'static>;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for UserGeometry {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl UserGeometry {
    pub fn new(device: &Device) -> Result<Self, Error> {
        Ok(Self(Geometry::new(device, GeometryKind::USER)?))
    }

    /// Sets a callback to query the bounding box of user-defined primitives.
    ///
    /// Only a single callback function can be registered per geometry, and
    /// further invocations overwrite the previously set callback function.
    /// Passing `None` as function pointer disables the registered callback
    /// function.
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
            D: UserData,
            F: FnMut(Option<&mut D>, u32, u32, &mut Bounds),
    {
        match self.kind {
            GeometryKind::USER => unsafe {
                let mut state = self.state.lock().unwrap();
                let mut closure = bounds;
                state.user_data.user_bounds_payload =
                    &mut closure as *mut _ as *mut std::os::raw::c_void;
                sys::rtcSetGeometryBoundsFunction(
                    self.handle,
                    crate::callback::user_bounds_function_helper(&mut closure),
                    ptr::null_mut(),
                );
            },
            _ => panic!("Only user-defined geometries can have a bounds function"),
        }
    }

    // TODO(yang): deal with RTCRayHitN, then we can make this function safe
    /// Sets the callback function to intersect a user geometry.
    ///
    /// Only a single callback function can be registered per geometry and
    /// further invocations overwrite the previously set callback function.
    /// Passing `None` as function pointer disables the registered callback
    /// function.
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
    pub unsafe fn set_intersect_function<F, D>(&mut self, intersect: F)
        where
            D: UserData,
            F: FnMut(&mut [i32], Option<&mut D>, u32, u32, &mut IntersectContext, &mut RTCRayHitN, u32),
    {
        // TODO: deal with RTCRayHitN
        let mut state = self.state.lock().unwrap();
        let mut closure = intersect;
        state.user_data.user_intersect_payload =
            &mut closure as *mut _ as *mut std::os::raw::c_void;
        sys::rtcSetGeometryIntersectFunction(
            self.handle,
            crate::callback::user_intersect_function_helper(&mut closure),
        );
    }

    // TODO(yang): deal with RTCRayN, then we can make this function safe
    /// Sets the callback function to occlude a user geometry.
    ///
    /// Similar to [`Geometry::set_user_intersect_function`], but for occlusion
    /// queries.
    pub unsafe fn set_occluded_function<F, D>(&mut self, occluded: F)
        where
            D: UserData,
            F: FnMut(&mut [i32], Option<&mut D>, u32, u32, &mut IntersectContext, &mut RTCRayN, u32),
    {
        // TODO: deal with RTCRayN
        let mut state = self.state.lock().unwrap();
        let mut closure = occluded;
        state.user_data.user_occluded_payload =
            &mut closure as *mut _ as *mut std::os::raw::c_void;
        sys::rtcSetGeometryOccludedFunction(
            self.handle,
            crate::callback::user_occluded_function_helper(&mut closure),
        );
    }

    /// Sets the number of primitives of a user-defined geometry.
    pub fn set_user_primitive_count(&mut self, count: u32) {
        // Update the primitive count.
        unsafe {
            sys::rtcSetGeometryUserPrimitiveCount(self.handle, count);
        }
    }
}