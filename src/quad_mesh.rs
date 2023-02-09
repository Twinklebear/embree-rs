use std::sync::Arc;

use crate::buffer::Buffer;
use crate::device::Device;
use crate::geometry::Geometry;
use crate::sys::*;
use crate::{BufferType, Format, GeometryType};

pub struct QuadMesh {
    device: Device,
    pub(crate) handle: RTCGeometry,
    pub vertex_buffer: Buffer<[f32; 4]>,
    pub index_buffer: Buffer<[u32; 4]>,
}

impl QuadMesh {
    pub fn unanimated(device: Device, num_quads: usize, num_verts: usize) -> Arc<QuadMesh> {
        let h = unsafe { rtcNewGeometry(device.handle, GeometryType::QUAD) };
        let mut vertex_buffer = Buffer::new(device.clone(), num_verts);
        let mut index_buffer = Buffer::new(device.clone(), num_quads);
        unsafe {
            rtcSetGeometryBuffer(
                h,
                BufferType::VERTEX,
                0,
                Format::FLOAT3,
                vertex_buffer.handle,
                0,
                16,
                num_verts,
            );
            vertex_buffer.set_attachment(h, BufferType::VERTEX, 0);

            rtcSetGeometryBuffer(
                h,
                BufferType::INDEX,
                0,
                Format::UINT4,
                index_buffer.handle,
                0,
                16,
                num_quads,
            );
            index_buffer.set_attachment(h, BufferType::INDEX, 0);
        }
        Arc::new(QuadMesh {
            device: device,
            handle: h,
            vertex_buffer: vertex_buffer,
            index_buffer: index_buffer,
        })
    }
}

impl Geometry for QuadMesh {
    fn handle(&self) -> RTCGeometry {
        self.handle
    }
}

impl Drop for QuadMesh {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseGeometry(self.handle);
        }
    }
}
