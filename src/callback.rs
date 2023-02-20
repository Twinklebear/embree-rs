use crate::{geometry::GeometryUserData, Bounds, IntersectContext, UserData};
use std::os::raw::c_void;

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
    D: UserData,
    F: FnMut(&mut [i32], Option<&mut D>, u32, u32, &mut IntersectContext, &mut RTCRayHitN, u32),
{
    unsafe extern "C" fn inner<F, D>(args: *const RTCIntersectFunctionNArguments)
    where
        F: FnMut(&mut [i32], Option<&mut D>, u32, u32, &mut IntersectContext, &mut RTCRayHitN, u32),
    {
        let cb_ptr =
            (*((*args).geometryUserPtr as *mut GeometryUserData)).user_intersect_payload as *mut F;
        if !cb_ptr.is_null() {
            let cb = &mut *cb_ptr;
            let user_data = {
                let user_data_ptr = (*((*args).geometryUserPtr as *mut GeometryUserData)).data;
                user_data_ptr
                    .is_null()
                    .then(|| &mut *(user_data_ptr as *mut D))
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
    D: UserData,
    F: FnMut(&mut [i32], Option<&mut D>, u32, u32, &mut IntersectContext, &mut RTCRayN, u32),
{
    unsafe extern "C" fn inner<F, D>(args: *const RTCOccludedFunctionNArguments)
    where
        F: FnMut(&mut [i32], Option<&mut D>, u32, u32, &mut IntersectContext, &mut RTCRayN, u32),
    {
        let cb_ptr =
            (*((*args).geometryUserPtr as *mut GeometryUserData)).user_occluded_payload as *mut F;
        if !cb_ptr.is_null() {
            let cb = &mut *cb_ptr;
            let user_data = {
                let user_data_ptr = (*((*args).geometryUserPtr as *mut GeometryUserData)).data;
                user_data_ptr
                    .is_null()
                    .then(|| &mut *(user_data_ptr as *mut D))
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
    D: UserData,
    F: FnMut(&mut [i32], Option<&mut D>, &mut IntersectContext, &mut RTCRayN, &mut RTCHitN, u32),
{
    unsafe extern "C" fn inner<F, D>(args: *const RTCFilterFunctionNArguments)
    where
        F: FnMut(
            &mut [i32],
            Option<&mut D>,
            &mut IntersectContext,
            &mut RTCRayN,
            &mut RTCHitN,
            u32,
        ),
    {
        let cb_ptr = (*((*args).geometryUserPtr as *mut GeometryUserData)).intersect_filter_payload
            as *mut F;
        if !cb_ptr.is_null() {
            let cb = &mut *cb_ptr;
            let user_data = {
                let user_data_ptr = (*((*args).geometryUserPtr as *mut GeometryUserData)).data;
                user_data_ptr
                    .is_null()
                    .then(|| &mut *(user_data_ptr as *mut D))
            };
            cb(
                std::slice::from_raw_parts_mut((*args).valid, (*args).N as usize),
                user_data,
                &mut *(*args).context,
                &mut *(*args).ray,
                &mut *(*args).hit,
                (*args).N,
            );
        }
    }

    Some(inner::<F, D>)
}

/// Helper function to convert a Rust closure to `RTCFilterFunctionN` callback
/// for occuluded.
pub fn occluded_filter_function_helper<F, D>(_f: &mut F) -> RTCFilterFunctionN
where
    D: UserData,
    F: FnMut(&mut [i32], Option<&mut D>, &mut IntersectContext, &mut RTCRayN, &mut RTCHitN, u32),
{
    unsafe extern "C" fn inner<F, D>(args: *const RTCFilterFunctionNArguments)
    where
        F: FnMut(
            &mut [i32],
            Option<&mut D>,
            &mut IntersectContext,
            &mut RTCRayN,
            &mut RTCHitN,
            u32,
        ),
    {
        let cb_ptr =
            (*((*args).geometryUserPtr as *mut GeometryUserData)).occluded_filter_payload as *mut F;
        if !cb_ptr.is_null() {
            let cb = &mut *cb_ptr;
            let user_data = {
                let user_data_ptr = (*((*args).geometryUserPtr as *mut GeometryUserData)).data;
                user_data_ptr
                    .is_null()
                    .then(|| &mut *(user_data_ptr as *mut D))
            };
            cb(
                std::slice::from_raw_parts_mut((*args).valid, (*args).N as usize),
                user_data,
                &mut *(*args).context,
                &mut *(*args).ray,
                &mut *(*args).hit,
                (*args).N,
            );
        }
    }

    Some(inner::<F, D>)
}

/// Helper function to convert a Rust closure to `RTCBoundsFunction` callback.
pub fn user_bounds_function_helper<F, D>(_f: &mut F) -> RTCBoundsFunction
where
    D: UserData,
    F: FnMut(Option<&mut D>, u32, u32, &mut Bounds),
{
    unsafe extern "C" fn inner<F, D>(args: *const RTCBoundsFunctionArguments)
    where
        F: FnMut(Option<&mut D>, u32, u32, &mut Bounds),
    {
        let cb_ptr =
            (*((*args).geometryUserPtr as *mut GeometryUserData)).user_bounds_payload as *mut F;
        if !cb_ptr.is_null() {
            let cb = &mut *cb_ptr;
            let user_data = {
                let user_data_ptr = (*((*args).geometryUserPtr as *mut GeometryUserData)).data;
                user_data_ptr
                    .is_null()
                    .then(|| &mut *(user_data_ptr as *mut D))
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
