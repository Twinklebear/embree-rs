use cgmath::{Vector2, Vector3, Vector4};

use buffer::Buffer;
use device::Device;
use geometry::Geometry;
use sys::*;
use {BufferType, Format, GeometryType, CurveType};

pub struct LinearCurve<'a> {
    device: &'a Device,
    pub(crate) handle: RTCGeometry,
    pub vertex_buffer: Buffer<'a, Vector4<f32>>,
    pub index_buffer: Buffer<'a, u32>,
    pub flag_buffer: Buffer<'a, u32>,
}

impl<'a> LinearCurve<'a> {
    pub fn unanimated(device: &'a Device, num_segments: usize, num_verts: usize, curve_type: CurveType) -> LinearCurve<'a> {
        let h: RTCGeometry;
        match curve_type {
        _ => h = unsafe { rtcNewGeometry(device.handle, GeometryType::FLAT_LINEAR_CURVE) },
        };
        let mut vertex_buffer = Buffer::new(device, num_verts);
        let mut index_buffer = Buffer::new(device, num_segments);
        let mut flag_buffer = Buffer::new(device, num_verts);
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
                num_segments,
            );
            index_buffer.set_attachment(h, BufferType::INDEX, 0);

            rtcSetGeometryBuffer(
                h,
                BufferType::FLAGS,
                0,
                Format::UCHAR,
                flag_buffer.handle,
                0,
                1,
                num_verts,
            );
            flag_buffer.set_attachment(h, BufferType::FLAGS, 0);
        }
        LinearCurve {
            device: device,
            handle: h,
            vertex_buffer: vertex_buffer,
            index_buffer: index_buffer,
            flag_buffer: flag_buffer,
        }
    }
}

unsafe impl<'a> Sync for LinearCurve<'a> {}

