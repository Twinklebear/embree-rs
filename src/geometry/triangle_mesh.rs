use crate::{sys::*, BufferGeometry, BufferUsage, Device, Format, Geometry, GeometryType};
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct TriangleMesh(BufferGeometry<'static>);

impl Deref for TriangleMesh {
    type Target = BufferGeometry<'static>;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for TriangleMesh {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl TriangleMesh {
    /// Creates a new triangle mesh geometry with the given number of triangles
    /// and vertices.
    ///
    /// The geometry is unanimated, and the vertex and index buffers are
    /// allocated but not initialized. The vertex buffer is in
    /// [`Format::FLOAT3`] format, and the index buffer is in
    /// [`Format::UINT3`] format, and are 16-byte and 4-byte aligned,
    /// respectively. They are bound to the first slot of their respective
    /// buffers.
    pub fn unanimated(device: &Device, num_tris: usize, num_verts: usize) -> TriangleMesh {
        let mut geometry = BufferGeometry::new(device, GeometryType::TRIANGLE).unwrap();
        let _ = geometry
            .set_new_buffer(BufferUsage::VERTEX, 0, Format::FLOAT3, 16, num_verts)
            .unwrap();
        let _ = geometry
            .set_new_buffer(BufferUsage::INDEX, 0, Format::UINT3, 12, num_tris)
            .unwrap();
        Self(geometry)
    }
}

impl Geometry for TriangleMesh {
    fn new(device: &Device) -> Result<TriangleMesh, RTCError>
    where
        Self: Sized,
    {
        Ok(Self(BufferGeometry::new(device, GeometryType::TRIANGLE)?))
    }

    fn kind(&self) -> GeometryType { GeometryType::TRIANGLE }

    fn handle(&self) -> RTCGeometry { self.handle }
}
