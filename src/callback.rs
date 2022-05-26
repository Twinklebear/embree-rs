use std::os::raw::c_void;

use crate::sys::*;

/// Helper function to convert a Rust closure to `RTCProgressMonitorFunction` callback.
pub(crate) fn progress_monitor_function_helper<F>(_f: &mut F) -> RTCProgressMonitorFunction
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

    inner::<F>
}

/// Helper function to convert a Rust closure to `RTCErrorFunction` callback.
pub(crate) fn error_function_helper<F>(_f: &mut F) -> RTCErrorFunction
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

    inner::<F>
}

/// Helper function to convert a Rust closure to `RTCMemoryMonitorFunction` callback.
pub(crate) fn memory_monitor_function_helper<F>(_f: &mut F) -> RTCMemoryMonitorFunction
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

    inner::<F>
}
