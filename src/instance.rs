use std::os::raw;

use cgmath::Matrix4;

use crate::device::Device;
use crate::scene::CommittedScene;
use crate::sys::*;
use crate::{Format, GeometryType};

pub struct Instance<'a> {
    device: &'a Device,
    pub(crate) handle: RTCGeometry,
    /// The scene being instanced
    scene: &'a CommittedScene<'a>,
}

impl<'a> Instance<'a> {
    pub fn unanimated(device: &'a Device, scene: &'a CommittedScene) -> Instance<'a> {
        let h = unsafe { rtcNewGeometry(device.handle, GeometryType::INSTANCE) };
        unsafe {
            rtcSetGeometryInstancedScene(h, scene.scene.handle);
        }
        Instance {
            device: device,
            handle: h,
            scene: scene,
        }
    }
    pub fn set_transform(&mut self, transform: &Matrix4<f32>) {
        let mat: &[f32; 16] = transform.as_ref();
        // Will this be fine if we don't set the number of timesteps? Default should be 1?
        unsafe {
            rtcSetGeometryTransform(
                self.handle,
                0,
                Format::FLOAT4X4_COLUMN_MAJOR,
                mat.as_ptr() as *const raw::c_void,
            );
        }
    }
}

unsafe impl<'a> Sync for Instance<'a> {}
