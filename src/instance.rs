use std::os::raw::c_uint;

use sys::*;
use ::Scene;

pub struct Instance<'a> {
    /// The scene we're instanced in
    scene: &'a Scene<'a>,
    /// The scene being instanced
    instance: &'a Scene<'a>,
    handle: c_uint,
}

impl<'a> Instance<'a> {
    pub fn unanimated(scene: &'a Scene, instance: &'a Scene) -> Instance<'a> {
        let h = unsafe {
            rtcNewInstance2(*scene.handle.borrow_mut(), *instance.handle.borrow(), 1)
        };
        Instance {
            scene: scene,
            instance: instance,
            handle: h
        }
    }
}

impl<'a> Drop for Instance<'a> {
    fn drop(&mut self) {
        unsafe {
            rtcDeleteGeometry(*self.scene.handle.borrow_mut(), self.handle);
        }
    }
}

