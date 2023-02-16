use std::{os::raw, sync::Arc};

use crate::{scene::Scene, sys::*, Format, GeometryType};

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
        Arc::new(Instance { handle: h, scene })
    }
    pub fn set_transform<Transform: AsRef<[f32; 16]>>(&mut self, transform: Transform) {
        let mat: &[f32; 16] = transform.as_ref();
        // Will this be fine if we don't set the number of timesteps? Default should be
        // 1?
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

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseGeometry(self.handle);
        }
    }
}
