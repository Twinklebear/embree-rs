use crate::{
    geometry::GeometryData, Bounds, Geometry, HitN, IntersectContext, RayN, UserGeometryData,
};
use std::{
    any::{Any, TypeId},
    os::raw::c_void,
};

use crate::sys::*;

/// Helper function to convert a Rust closure to `RTCProgressMonitorFunction`
/// callback.
pub fn progress_monitor_function_helper<F>(_f: &mut F) -> RTCProgressMonitorFunction
where
    F: FnMut(f64) -> bool,
{
    unsafe extern "C" fn inner<F>(f: *mut c_void, n: f64) -> bool
    where
        F: FnMut(f64) -> bool,
    {
        let cb = &mut *(f as *mut F);
        cb(n)
    }

    Some(inner::<F>)
}

/// Helper function to convert a Rust closure to `RTCErrorFunction` callback.
pub fn error_function_helper<F>(_f: &mut F) -> RTCErrorFunction
where
    F: FnMut(RTCError, &'static str),
{
    unsafe extern "C" fn inner<F>(
        f: *mut c_void,
        error: RTCError,
        msg: *const ::std::os::raw::c_char,
    ) where
        F: FnMut(RTCError, &'static str),
    {
        let cb = &mut *(f as *mut F);
        cb(error, std::ffi::CStr::from_ptr(msg).to_str().unwrap())
    }

    Some(inner::<F>)
}

/// Helper function to convert a Rust closure to `RTCMemoryMonitorFunction`
/// callback.
pub fn memory_monitor_function_helper<F>(_f: &mut F) -> RTCMemoryMonitorFunction
where
    F: FnMut(isize, bool) -> bool,
{
    unsafe extern "C" fn inner<F>(f: *mut c_void, bytes: isize, post: bool) -> bool
    where
        F: FnMut(isize, bool) -> bool,
    {
        let cb = &mut *(f as *mut F);
        cb(bytes, post)
    }

    Some(inner::<F>)
}

// TODO: deal with RTCRayHitN, convert it to a SOA struct
/// Helper function to convert a Rust closure to `RTCIntersectFunctionN`
/// callback.
pub fn user_intersect_function_helper<F, D>(_f: &mut F) -> RTCIntersectFunctionN
where
    D: UserGeometryData,
    F: FnMut(&mut [i32], Option<&mut D>, u32, u32, &mut IntersectContext, &mut RTCRayHitN, u32),
{
    unsafe extern "C" fn inner<F, D>(args: *const RTCIntersectFunctionNArguments)
    where
        D: UserGeometryData,
        F: FnMut(&mut [i32], Option<&mut D>, u32, u32, &mut IntersectContext, &mut RTCRayHitN, u32),
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
                        if user_data.data.is_null() || user_data.data.type_id() != TypeId::of::<D>()
                        {
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
                &mut *(*args).rayhit,
                (*args).N,
            );
        }
    }

    Some(inner::<F, D>)
}

// TODO: deal with RTCRayN
/// Helper function to convert a Rust closure to `RTCOccludedFunctionN`
/// callback.
pub fn user_occluded_function_helper<F, D>(_f: &mut F) -> RTCOccludedFunctionN
where
    D: UserGeometryData,
    F: FnMut(&mut [i32], Option<&mut D>, u32, u32, &mut IntersectContext, &mut RTCRayN, u32),
{
    unsafe extern "C" fn inner<F, D>(args: *const RTCOccludedFunctionNArguments)
    where
        D: UserGeometryData,
        F: FnMut(&mut [i32], Option<&mut D>, u32, u32, &mut IntersectContext, &mut RTCRayN, u32),
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
                        if user_data.data.is_null() || user_data.data.type_id() != TypeId::of::<D>()
                        {
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
                &mut *(*args).ray,
                (*args).N,
            )
        }
    }

    Some(inner::<F, D>)
}

/// Helper function to convert a Rust closure to `RTCFilterFunctionN` callback
/// for intersect.
pub fn intersect_filter_function_helper<F, D>(_f: &mut F) -> RTCFilterFunctionN
where
    D: UserGeometryData,
    F: FnMut(&mut [i32], Option<&mut D>, &mut IntersectContext, RayN, HitN),
{
    unsafe extern "C" fn inner<F, D>(args: *const RTCFilterFunctionNArguments)
    where
        D: UserGeometryData,
        F: FnMut(&mut [i32], Option<&mut D>, &mut IntersectContext, RayN, HitN),
    {
        let cb_ptr =
            (*((*args).geometryUserPtr as *mut GeometryData)).intersect_filter_fn as *mut F;
        if !cb_ptr.is_null() {
            let cb = &mut *cb_ptr;
            let user_data = {
                match (*((*args).geometryUserPtr as *mut GeometryData)).user_data {
                    Some(ref user_data) => {
                        if user_data.data.is_null() || user_data.data.type_id() != TypeId::of::<D>()
                        {
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
                &mut *(*args).context,
                RayN {
                    ptr: &mut *(*args).ray,
                    len: (*args).N as usize,
                },
                HitN {
                    ptr: &mut *(*args).hit,
                    len: (*args).N as usize,
                },
            );
        }
    }

    Some(inner::<F, D>)
}

/// Helper function to convert a Rust closure to `RTCFilterFunctionN` callback
/// for occuluded.
pub fn occluded_filter_function_helper<F, D>(_f: &mut F) -> RTCFilterFunctionN
where
    D: UserGeometryData,
    F: FnMut(&mut [i32], Option<&mut D>, &mut IntersectContext, RayN, HitN),
{
    unsafe extern "C" fn inner<F, D>(args: *const RTCFilterFunctionNArguments)
    where
        D: UserGeometryData,
        F: FnMut(&mut [i32], Option<&mut D>, &mut IntersectContext, RayN, HitN),
    {
        let len = (*args).N as usize;
        let cb_ptr = (*((*args).geometryUserPtr as *mut GeometryData)).occluded_filter_fn as *mut F;
        if !cb_ptr.is_null() {
            let cb = &mut *cb_ptr;
            let user_data = {
                match (*((*args).geometryUserPtr as *mut GeometryData)).user_data {
                    Some(ref user_data) => {
                        if user_data.data.is_null() || user_data.data.type_id() != TypeId::of::<D>()
                        {
                            None
                        } else {
                            Some(&mut *(user_data.data as *mut D))
                        }
                    }
                    None => None,
                }
            };
            cb(
                std::slice::from_raw_parts_mut((*args).valid, len),
                user_data,
                &mut *(*args).context,
                RayN {
                    ptr: &mut *(*args).ray,
                    len,
                },
                HitN {
                    ptr: &mut *(*args).hit,
                    len,
                },
            );
        }
    }

    Some(inner::<F, D>)
}

/// Helper function to convert a Rust closure to `RTCBoundsFunction` callback.
pub fn user_bounds_function_helper<F, D>(_f: &mut F) -> RTCBoundsFunction
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
                        if user_data.data.is_null() || user_data.data.type_id() != TypeId::of::<D>()
                        {
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

/// Helper function to convert a Rust closure to `RTCDisplacementFunctionN`
/// callback.
pub fn subdivision_displacement_function_helper<F, D>(_f: &mut F) -> RTCDisplacementFunctionN
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
                        if user_data.data.is_null() || user_data.data.type_id() != TypeId::of::<D>()
                        {
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

// TODO: point query function helper
