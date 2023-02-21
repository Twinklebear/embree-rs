use crate::{sys, BuildFlags, BuildQuality, Device, Error};

pub struct Bvh {
    handle: sys::RTCBVH,
}

impl Clone for Bvh {
    fn clone(&self) -> Self {
        unsafe { sys::rtcRetainBVH(self.handle) }
        Self {
            handle: self.handle,
        }
    }
}

impl Drop for Bvh {
    fn drop(&mut self) { unsafe { sys::rtcReleaseBVH(self.handle) } }
}

impl Bvh {
    pub(crate) fn new(device: &Device) -> Result<Self, Error> {
        let handle = unsafe { sys::rtcNewBVH(device.handle) };
        if handle.is_null() {
            Err(device.get_error())
        } else {
            Ok(Self { handle })
        }
    }

    // TODO: BVH build functions
}
