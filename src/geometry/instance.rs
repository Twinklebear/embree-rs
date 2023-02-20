use crate::{sys,
    Device, Error, Format, Geometry, GeometryKind, QuaternionDecomposition, Scene,
};
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct Instance(Geometry<'static>);

impl Deref for Instance {
    type Target = Geometry<'static>;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for Instance {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl Instance {
    pub fn new(device: &Device) -> Result<Self, Error> {
        Ok(Self(Geometry::new(device, GeometryKind::INSTANCE)?))
    }

    /// Sets the instanced scene of an instance geometry.
    pub fn set_instanced_scene(&mut self, scene: &Scene) {
        unsafe { sys::rtcSetGeometryInstancedScene(self.handle, scene.handle) }
    }

    // TODO(yang): Better transform type
    /// Returns the interpolated instance transformation for the specified time
    /// step.
    pub fn get_geometry_transform(&mut self, time: f32, format: Format) -> [f32; 16] {
        unsafe {
            let mut transform = [0.0; 16];
            sys::rtcGetGeometryTransform(
                self.handle,
                time,
                format,
                transform.as_mut_ptr() as *mut _,
            );
            transform
        }
    }

    // TODO(yang): Better transform type
    /// Sets the transformation for a particular time step of an instance
    /// geometry.
    pub fn set_geometry_transform(&mut self, time_step: u32, format: Format, transform: &[f32]) {
        unsafe {
            sys::rtcSetGeometryTransform(
                self.handle,
                time_step,
                format,
                transform.as_ptr() as *const _,
            );
        }
    }

    /// Sets the transformation for a particular time step of an instance
    /// geometry as a decomposition of the transformation matrix using
    /// quaternions to represent the rotation.
    pub fn set_transform_quaternion(
        &mut self,
        time_step: u32,
        transform: &QuaternionDecomposition,
    ) {
        unsafe {
            sys::rtcSetGeometryTransformQuaternion(
                self.handle,
                time_step,
                transform as &QuaternionDecomposition as *const _,
            );
        }
    }
}
