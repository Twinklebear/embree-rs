use std::os::raw;

use cgmath::Matrix4;

use sys::*;
use device::Device;
use scene::Scene;
use geometry::Geometry;
use ::{Format, GeometryType, BufferType};

pub struct Instance<'a> {
    device: &'a Device,
    pub(crate) handle: RTCGeometry,
    /// The scene being instanced
    scene: &'a Scene<'a>,
}

impl<'a> Instance<'a> {
    pub fn unanimated(device: &'a Device, scene: &'a Scene) -> Instance<'a> {
        let h = unsafe { rtcNewGeometry(device.handle, GeometryType::INSTANCE) };
        unsafe { rtcSetGeometryInstancedScene(h, scene.handle); }
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
            rtcSetGeometryTransform(self.handle, 0,
                                    Format::FLOAT4X4_COLUMN_MAJOR,
                                    mat.as_ptr() as *const raw::c_void);
        }
    }
}

impl<'a> Drop for Instance<'a> {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseGeometry(self.handle);
        }
    }
}

impl<'a> Geometry for Instance<'a> {
    fn handle(&self) -> RTCGeometry { self.handle }
}

