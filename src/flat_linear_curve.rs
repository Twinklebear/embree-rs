use cgmath::{Vector2, Vector3, Vector4};

use buffer::Buffer;
use device::Device;
use geometry::Geometry;
use sys::*;
use {BufferType, Format, GeometryType};

pub struct FlatLinearCurve<'a> {
    device: &'a Device,
    pub(crate) handle: RTCGeometry,
    pub vertex_buffer: Buffer<'a, Vector4<f32>>,
    pub index_buffer: Buffer<'a, u32>,
}

impl<'a> FlatLinearCurve<'a> {
    pub fn unanimated(device: &'a Device, num_tris: usize, num_verts: usize) -> FlatLinearCurve<'a> {
        let h = unsafe { rtcNewGeometry(device.handle, GeometryType::FLAT_LINEAR_CURVE) };
        let mut vertex_buffer = Buffer::new(device, num_verts);
        let mut index_buffer = Buffer::new(device, num_tris);
        unsafe {
            rtcSetGeometryBuffer(
                h,
                BufferType::VERTEX,
                0,
                Format::FLOAT4,
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
                Format::UINT,
                index_buffer.handle,
                0,
                4,
                num_tris,
            );
            index_buffer.set_attachment(h, BufferType::INDEX, 0);
        }
        FlatLinearCurve {
            device: device,
            handle: h,
            vertex_buffer: vertex_buffer,
            index_buffer: index_buffer,
        }
    }
}

unsafe impl<'a> Sync for FlatLinearCurve<'a> {}

