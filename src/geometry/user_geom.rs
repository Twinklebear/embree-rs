use crate::{
    geometry::GeometryData, sys::*, Bounds, Device, Error, Geometry, GeometryKind,
    IntersectContext, RayHitN, RayN, UserGeometryData,
};
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
    ptr,
};

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
    ///
    /// Unregister the callback function by calling
    /// [`UserGeometry::unset_bounds_function`].
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
        D: UserGeometryData,
        F: FnMut(Option<&mut D>, u32, u32, &mut Bounds),
    {
        unsafe {
            let mut state = self.state.lock().unwrap();
            let mut closure = bounds;
            state.data.user_fns.as_mut().unwrap().bounds_fn =
                &mut closure as *mut _ as *mut std::os::raw::c_void;
            rtcSetGeometryBoundsFunction(
                self.handle,
                bounds_function(&mut closure),
                ptr::null_mut(),
            );
        }
    }

    /// Unsets the callback to calculate the bounding box of user-defined
    /// geometry.
    pub fn unset_bounds_function(&mut self) {
        unsafe {
            rtcSetGeometryBoundsFunction(self.handle, None, ptr::null_mut());
        }
    }

    /// Sets the callback function to intersect a user geometry.
    ///
    /// Only a single callback function can be registered per geometry and
    /// further invocations overwrite the previously set callback function.
    /// Unregister the callback function by calling
    /// [`UserGeometry::unset_intersect_function`].
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
    pub fn set_intersect_function<F, D>(&mut self, intersect: F)
    where
        D: UserGeometryData,
        F: FnMut(&mut [i32], Option<&mut D>, u32, u32, &mut IntersectContext, RayHitN),
    {
        let mut state = self.state.lock().unwrap();
        let mut closure = intersect;
        state.data.user_fns.as_mut().unwrap().intersect_fn =
            &mut closure as *mut _ as *mut std::os::raw::c_void;
        unsafe { rtcSetGeometryIntersectFunction(self.handle, intersect_function(&mut closure)) };
    }

    /// Unsets the callback to intersect user-defined geometry.
    pub fn unset_intersect_function(&mut self) {
        unsafe {
            rtcSetGeometryIntersectFunction(self.handle, None);
        }
    }

    /// Sets the callback function to occlude a user geometry.
    ///
    /// Similar to [`Geometry::set_intersect_function`], but for occlusion
    /// queries.
    pub fn set_occluded_function<F, D>(&mut self, occluded: F)
    where
        D: UserGeometryData,
        F: FnMut(&mut [i32], Option<&mut D>, u32, u32, &mut IntersectContext, RayN),
    {
        let mut state = self.state.lock().unwrap();
        let mut closure = occluded;
        state.data.user_fns.as_mut().unwrap().occluded_fn =
            &mut closure as *mut _ as *mut std::os::raw::c_void;
        unsafe { rtcSetGeometryOccludedFunction(self.handle, occluded_function(&mut closure)) };
    }

    /// Unsets the callback to occlude user-defined geometry.
    pub fn unset_occluded_function(&mut self) {
        unsafe {
            rtcSetGeometryOccludedFunction(self.handle, None);
        }
    }

    /// Sets the number of primitives of a user-defined geometry.
    pub fn set_primitive_count(&mut self, count: u32) {
        // Update the primitive count.
        unsafe {
            rtcSetGeometryUserPrimitiveCount(self.handle, count);
        }
    }
}

/// Helper function to convert a Rust closure to `RTCBoundsFunction` callback.
fn bounds_function<F, D>(_f: &mut F) -> RTCBoundsFunction
where
    D: UserGeometryData,
    F: FnMut(Option<&mut D>, u32, u32, &mut Bounds),
{
    unsafe extern "C" fn inner<F, D>(args: *const RTCBoundsFunctionArguments)
    where
        D: UserGeometryData,
        F: FnMut(Option<&mut D>, u32, u32, &mut Bounds),
    {
        let cb_ptr = (*((*args).geometryUserPtr as *mut GeometryData))
            .user_fns
            .as_ref()
            .expect(
                "User payloads not set! Make sure the geometry was created with kind \
                 GeometryKind::USER",
            )
            .bounds_fn as *mut F;
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
                (*args).primID,
                (*args).timeStep,
                &mut *(*args).bounds_o,
            );
        }
    }

    Some(inner::<F, D>)
}

/// Helper function to convert a Rust closure to `RTCIntersectFunctionN`
/// callback.
fn intersect_function<F, D>(_f: &mut F) -> RTCIntersectFunctionN
where
    D: UserGeometryData,
    F: FnMut(&mut [i32], Option<&mut D>, u32, u32, &mut IntersectContext, RayHitN),
{
    unsafe extern "C" fn inner<F, D>(args: *const RTCIntersectFunctionNArguments)
    where
        D: UserGeometryData,
        F: FnMut(&mut [i32], Option<&mut D>, u32, u32, &mut IntersectContext, RayHitN),
    {
        let cb_ptr = (*((*args).geometryUserPtr as *mut GeometryData))
            .user_fns
            .as_ref()
            .expect(
                "User payloads not set! Make sure the geometry was created with kind \
                 GeometryKind::USER",
            )
            .intersect_fn as *mut F;
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
                std::slice::from_raw_parts_mut((*args).valid, (*args).N as usize),
                user_data,
                (*args).geomID,
                (*args).primID,
                &mut *(*args).context,
                RayHitN {
                    ptr: (*args).rayhit,
                    len: (*args).N as usize,
                },
            );
        }
    }

    Some(inner::<F, D>)
}

/// Helper function to convert a Rust closure to `RTCOccludedFunctionN`
/// callback.
fn occluded_function<F, D>(_f: &mut F) -> RTCOccludedFunctionN
where
    D: UserGeometryData,
    F: FnMut(&mut [i32], Option<&mut D>, u32, u32, &mut IntersectContext, RayN),
{
    unsafe extern "C" fn inner<F, D>(args: *const RTCOccludedFunctionNArguments)
    where
        D: UserGeometryData,
        F: FnMut(&mut [i32], Option<&mut D>, u32, u32, &mut IntersectContext, RayN),
    {
        let cb_ptr = (*((*args).geometryUserPtr as *mut GeometryData))
            .user_fns
            .as_ref()
            .expect(
                "User payloads not set! Make sure the geometry was created with kind \
                 GeometryKind::USER",
            )
            .occluded_fn as *mut F;
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
                std::slice::from_raw_parts_mut((*args).valid, (*args).N as usize),
                user_data,
                (*args).geomID,
                (*args).primID,
                &mut *(*args).context,
                RayN {
                    ptr: (*args).ray,
                    len: (*args).N as usize,
                },
            )
        }
    }

    Some(inner::<F, D>)
}
