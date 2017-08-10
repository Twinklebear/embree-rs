use std::cell::RefCell;
use std::marker::PhantomData;
use std::mem;

use sys::*;
use device::Device;
use ::{SceneFlags, AlgorithmFlags};

pub struct Scene<'a> {
    pub(crate) handle: RefCell<RTCScene>,
    /// We don't need to actually keep a reference to the device,
    /// we just need to track its lifetime for correctness
    device: PhantomData<&'a Device>,
}
impl<'a> Scene<'a> {
    pub fn new(device: &'a Device, scene_flags: SceneFlags,
               algorithm_flags: AlgorithmFlags) -> Scene
    {
        let h = unsafe {
            rtcDeviceNewScene(device.handle,
                              mem::transmute(scene_flags.bits),
                              mem::transmute(algorithm_flags.bits))
        };
        Scene { handle: RefCell::new(h), device: PhantomData }
    }
}
impl<'a> Drop for Scene<'a> {
    fn drop(&mut self) {
        unsafe { rtcDeleteScene(*self.handle.borrow_mut()); }
    }
}

