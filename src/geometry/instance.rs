use crate::{sys, Device, Error, Format, Geometry, GeometryKind, QuaternionDecomposition, Scene};
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

    /// Returns the interpolated instance transformation for the specified time
    /// step.
    ///
    /// The transformation is returned as a 4x4 column-major matrix.
    pub fn get_transform(&mut self, time: f32) -> [f32; 16] {
        unsafe {
            let mut transform = [0.0; 16];
            sys::rtcGetGeometryTransform(
                self.handle,
                time,
                Format::FLOAT4X4_COLUMN_MAJOR,
                transform.as_mut_ptr() as *mut _,
            );
            transform
        }
    }

    /// Sets the transformation for a particular time step of an instance
    /// geometry.
    ///
    /// The transformation is specified as a 4x4 column-major matrix.
    pub fn set_transform(&mut self, time_step: u32, transform: &[f32; 16]) {
        unsafe {
            sys::rtcSetGeometryTransform(
                self.handle,
                time_step,
                Format::FLOAT4X4_COLUMN_MAJOR,
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
