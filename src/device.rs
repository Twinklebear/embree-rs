use std::ptr;

use sys::*;

pub struct Device {
    handle: RTCDevice,
}
impl Device {
    pub fn new() -> Device {
        // TODO: Call and set the flush zero and denormals modes
        // as recommended by Embree
        Device { handle: unsafe { rtcNewDevice(ptr::null()) } }
    }
    // TODO: function that makes a device with a config string
}
impl Drop for Device {
    fn drop(&mut self) {
        unsafe { rtcDeleteDevice(self.handle); }
    }
}

