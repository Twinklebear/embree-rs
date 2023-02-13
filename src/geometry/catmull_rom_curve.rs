use std::sync::Arc;

use crate::buffer::Buffer;
use crate::device::Device;
use crate::geometry::GeometryTrait;
use crate::sys::*;
use crate::{BufferType, CurveType, Format, GeometryType};

pub struct CatmullRomCurve {
    device: Device,
    pub(crate) handle: RTCGeometry,
    pub vertex_buffer: Buffer<[f32; 4]>,
    pub index_buffer: Buffer<u32>,
    pub normal_buffer: Option<Buffer<[f32; 3]>>,
}

impl CatmullRomCurve {
    pub fn flat(
        device: Device,
        num_segments: usize,
        num_verts: usize,
        use_normals: bool,
    ) -> Arc<CatmullRomCurve> {
        CatmullRomCurve::unanimated(
            device,
            num_segments,
            num_verts,
            CurveType::Flat,
            use_normals,
        )
    }
    pub fn round(
        device: Device,
        num_segments: usize,
        num_verts: usize,
        use_normals: bool,
    ) -> Arc<CatmullRomCurve> {
        CatmullRomCurve::unanimated(
            device,
            num_segments,
            num_verts,
            CurveType::Round,
            use_normals,
        )
    }
    pub fn normal_oriented(
        device: Device,
        num_segments: usize,
        num_verts: usize,
    ) -> Arc<CatmullRomCurve> {
        CatmullRomCurve::unanimated(
            device,
            num_segments,
            num_verts,
            CurveType::NormalOriented,
            true,
        )
    }

    fn unanimated(
        device: Device,
        num_segments: usize,
        num_verts: usize,
        curve_type: CurveType,
        use_normals: bool,
    ) -> Arc<CatmullRomCurve> {
        let h: RTCGeometry;
        match curve_type {
            CurveType::NormalOriented => {
                h = unsafe {
                    rtcNewGeometry(
                        device.handle,
                        GeometryType::NORMAL_ORIENTED_CATMULL_ROM_CURVE,
                    )
                }
            }
            CurveType::Round => {
                h = unsafe { rtcNewGeometry(device.handle, GeometryType::ROUND_CATMULL_ROM_CURVE) }
            }
            _ => h = unsafe { rtcNewGeometry(device.handle, GeometryType::FLAT_CATMULL_ROM_CURVE) },
        };
        let mut vertex_buffer = Buffer::new(device.clone(), num_verts);
        let mut index_buffer = Buffer::new(device.clone(), num_segments);
        let mut normal_buffer = None;

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

            if use_normals {
                let mut temp_normal_buffer = Buffer::new(device.clone(), num_verts);
                rtcSetGeometryBuffer(
                    h,
                    BufferType::NORMAL,
                    0,
                    Format::FLOAT3,
                    temp_normal_buffer.handle,
                    0,
                    12,
                    num_verts,
                );
                temp_normal_buffer.set_attachment(h, BufferType::NORMAL, 0);
                normal_buffer = Some(temp_normal_buffer);
            }
        }
        Arc::new(CatmullRomCurve {
            device: device,
            handle: h,
            vertex_buffer: vertex_buffer,
            index_buffer: index_buffer,
            normal_buffer: normal_buffer,
        })
    }
}

impl GeometryTrait for CatmullRomCurve {
    fn handle(&self) -> RTCGeometry {
        self.handle
    }
}

impl Drop for CatmullRomCurve {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseGeometry(self.handle);
        }
    }
}
