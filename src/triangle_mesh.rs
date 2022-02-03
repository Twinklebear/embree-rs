use cgmath::{Vector3, Vector4};
use std::sync::Arc;

use crate::buffer::Buffer;
use crate::device::Device;
use crate::geometry::Geometry;
use crate::sys::*;
use crate::{BufferType, Format, GeometryType};

pub struct TriangleMesh {
    device: Arc<Device>,
    pub(crate) handle: RTCGeometry,
    pub vertex_buffer: Buffer<Vector4<f32>>,
    pub index_buffer: Buffer<Vector3<u32>>,
}

impl TriangleMesh {
    pub fn unanimated(device: Arc<Device>, num_tris: usize, num_verts: usize) -> Arc<TriangleMesh> {
        let h = unsafe { rtcNewGeometry(device.handle, GeometryType::TRIANGLE) };
        let mut vertex_buffer = Buffer::new(device.clone(), num_verts);
        let mut index_buffer = Buffer::new(device.clone(), num_tris);
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
                Format::UINT3,
                index_buffer.handle,
                0,
                12,
                num_tris,
            );
            index_buffer.set_attachment(h, BufferType::INDEX, 0);
        }
        Arc::new(TriangleMesh {
            device: device,
            handle: h,
            vertex_buffer: vertex_buffer,
            index_buffer: index_buffer,
        })
    }
}

impl Geometry for TriangleMesh {
    fn handle(&self) -> RTCGeometry {
        self.handle
    }
}

impl Drop for TriangleMesh {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseGeometry(self.handle);
        }
    }
}
