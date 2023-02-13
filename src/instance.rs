use std::{os::raw, sync::Arc};

use crate::{geometry::Geometry, scene::Scene, sys::*, Device, Format, GeometryType};

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

impl Geometry for Instance {
    fn new(device: &Device) -> Result<Self, RTCError>
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn handle(&self) -> RTCGeometry { self.handle }

    fn kind(&self) -> GeometryType { GeometryType::INSTANCE }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseGeometry(self.handle);
        }
    }
}
