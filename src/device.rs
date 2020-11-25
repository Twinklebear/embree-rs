use std::ffi::CString;
use std::ptr;

use sys::*;

pub struct Device {
    pub(crate) handle: RTCDevice,
}

impl Device {
    pub fn new() -> Device {
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
