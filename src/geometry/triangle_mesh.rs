use crate::sys::*;
use crate::{BufferUsage, Device, Format, GeometryType, BufferGeometry, Geometry};
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct TriangleMesh(BufferGeometry);

impl Deref for TriangleMesh {
    type Target = BufferGeometry;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TriangleMesh {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TriangleMesh {
    pub fn unanimated(device: &Device, num_tris: usize, num_verts: usize) -> TriangleMesh {
        let mut geometry = BufferGeometry::new(device, GeometryType::TRIANGLE).unwrap();

        geometry
            .set_new_buffer(BufferUsage::VERTEX, 0, Format::FLOAT3, 16, num_verts)
            .unwrap();
        geometry
            .set_new_buffer(BufferUsage::INDEX, 0, Format::UINT3, 12, num_tris)
            .unwrap();

        Self(geometry)
    }
}

impl Geometry for TriangleMesh {
    fn new(device: &Device) -> Result<Self, RTCError> where Self: Sized {
        Ok(Self(BufferGeometry::new(device, GeometryType::TRIANGLE)?))
    }

    fn kind(&self) -> GeometryType {
        GeometryType::TRIANGLE
    }

    fn handle(&self) -> RTCGeometry {
        self.handle
    }
}
