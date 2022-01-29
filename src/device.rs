#[cfg(x86_64)]
use std::arch::x86_64;
use std::ffi::CString;
use std::ptr;

use crate::sys::*;

pub struct Device {
    pub(crate) handle: RTCDevice,
}

impl Device {
    pub fn new() -> Device {
        // Set the flush zero and denormals modes from Embrees's perf. recommendations
        // https://embree.github.io/api.html#performance-recommendations
        // Though, in Rust I think we just call the below function to do both
        #[cfg(x86_64)]
        unsafe {
            x86_64::_MM_SET_FLUSH_ZERO_MODE(x86_64::_MM_FLUSH_ZERO_ON);
        }

        Device {
            handle: unsafe { rtcNewDevice(ptr::null()) },
        }
    }
    pub fn debug() -> Device {
        let cfg = CString::new("verbose=4").unwrap();
        Device {
            handle: unsafe { rtcNewDevice(cfg.as_ptr()) },
        }
    }
    // TODO: Setup the flush zero and denormals mode needed by Embree
    // using the Rust SIMD when it's in core
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseDevice(self.handle);
        }
    }
}

unsafe impl Sync for Device {}
