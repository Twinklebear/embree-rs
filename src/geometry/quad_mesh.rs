use crate::{
    sys, BufferGeometry, BufferUsage, Device, Error, Format, Geometry, GeometryType,
    GeometryVertexAttribute,
};
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct QuadMesh(BufferGeometry<'static>);

impl Deref for QuadMesh {
    type Target = BufferGeometry<'static>;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for QuadMesh {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl QuadMesh {
    /// Creates a new quad mesh geometry with the given number of quads and
    /// vertices.
    ///
    /// The geometry is unanimated, and the vertex and index buffers are
    /// allocated but not initialized. The vertex buffer is in
    /// [`Format::FLOAT3`] format, the index buffer is in
    /// [`Format::UINT4`] format. They are 16-byte and 4-byte aligned
    /// respectively, and are bound to the first slot of their respective
    /// buffers.
    pub fn unanimated<'a>(device: &'a Device, num_quads: usize, num_verts: usize) -> QuadMesh {
        let mut geometry = BufferGeometry::new(device, GeometryType::QUAD).unwrap();
        geometry
            .set_new_buffer(BufferUsage::VERTEX, 0, Format::FLOAT3, 16, num_verts)
            .unwrap();
        geometry
            .set_new_buffer(BufferUsage::INDEX, 0, Format::UINT4, 16, num_quads)
            .unwrap();
        Self(geometry)
    }
}

impl Geometry for QuadMesh {
    fn new(device: &Device) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Ok(Self(BufferGeometry::new(device, GeometryType::QUAD)?))
    }

    fn kind(&self) -> GeometryType { GeometryType::QUAD }

    fn handle(&self) -> sys::RTCGeometry { self.handle }
}

impl GeometryVertexAttribute for QuadMesh {}
