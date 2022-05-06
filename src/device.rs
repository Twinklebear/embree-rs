use std::ffi::CString;
use std::ptr;
use std::sync::Arc;

use crate::sys::*;

pub struct Device {
    pub(crate) handle: RTCDevice,
}

impl Device {
    pub fn new() -> Arc<Device> {
        // Set the flush zero and denormals modes from Embrees's perf. recommendations
        // https://embree.github.io/api.html#performance-recommendations
        // Though, in Rust I think we just call the below function to do both
        #[cfg(target_arch = "x86_64")]
        unsafe {
            use std::arch::x86_64;
            x86_64::_MM_SET_FLUSH_ZERO_MODE(x86_64::_MM_FLUSH_ZERO_ON);
        }

        Arc::new(Device {
            handle: unsafe { rtcNewDevice(ptr::null()) },
        })
    }

    pub fn debug() -> Arc<Device> {
        let cfg = CString::new("verbose=4").unwrap();
        Arc::new(Device {
            handle: unsafe { rtcNewDevice(cfg.as_ptr()) },
        })
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseDevice(self.handle);
        }
    }
}

unsafe impl Sync for Device {}
