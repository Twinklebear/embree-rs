use std::os::raw;
use std::sync::Arc;

use crate::geometry::GeometryTrait;
use crate::scene::Scene;
use crate::sys::*;
use crate::{Format, GeometryType};

pub struct Instance {
    /// The scene being instanced
    scene: Arc<Scene>,
    pub(crate) handle: RTCGeometry,
}

impl Instance {
    pub fn unanimated(scene: Arc<Scene>) -> Arc<Instance> {
        let h = unsafe { rtcNewGeometry(scene.device.handle, GeometryType::INSTANCE) };
        unsafe {
            rtcSetGeometryInstancedScene(h, scene.handle);
        }
        Arc::new(Instance {
            handle: h,
            scene: scene,
        })
    }
    pub fn set_transform<Transform: AsRef<[f32; 16]>>(&mut self, transform: Transform) {
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

impl GeometryTrait for Instance {
    fn handle(&self) -> RTCGeometry {
        self.handle
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseGeometry(self.handle);
        }
    }
}
