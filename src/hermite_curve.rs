use std::sync::Arc;

use crate::buffer::Buffer;
use crate::device::Device;
use crate::geometry::Geometry;
use crate::sys::*;
use crate::{BufferType, CurveType, Format, GeometryType};

pub struct HermiteCurve {
    device: Arc<Device>,
    pub(crate) handle: RTCGeometry,
    pub vertex_buffer: Buffer<[f32; 4]>,
    pub index_buffer: Buffer<u32>,
    pub tangent_buffer: Buffer<[f32; 4]>,
    pub normal_derivative_buffer: Option<Buffer<[f32; 3]>>,
    pub normal_buffer: Option<Buffer<[f32; 3]>>,
}

impl HermiteCurve {
    pub fn flat(
        device: Arc<Device>,
        num_segments: usize,
        num_verts: usize,
        use_normals: bool,
    ) -> Arc<HermiteCurve> {
        HermiteCurve::unanimated(
            device,
            num_segments,
            num_verts,
            CurveType::Flat,
            use_normals,
        )
    }
    pub fn round(
        device: Arc<Device>,
        num_segments: usize,
        num_verts: usize,
        use_normals: bool,
    ) -> Arc<HermiteCurve> {
        HermiteCurve::unanimated(
            device,
            num_segments,
            num_verts,
            CurveType::Round,
            use_normals,
        )
    }
    pub fn normal_oriented(
        device: Arc<Device>,
        num_segments: usize,
        num_verts: usize,
    ) -> Arc<HermiteCurve> {
        HermiteCurve::unanimated(
            device,
            num_segments,
            num_verts,
            CurveType::NormalOriented,
            true,
        )
    }

    fn unanimated(
        device: Arc<Device>,
        num_segments: usize,
        num_verts: usize,
        curve_type: CurveType,
        use_normals: bool,
    ) -> Arc<HermiteCurve> {
        let h: RTCGeometry;
        match curve_type {
            CurveType::NormalOriented => {
                h = unsafe {
                    rtcNewGeometry(device.handle, GeometryType::NORMAL_ORIENTED_HERMITE_CURVE)
                }
            }
            CurveType::Round => {
                h = unsafe { rtcNewGeometry(device.handle, GeometryType::ROUND_HERMITE_CURVE) }
            }
            _ => h = unsafe { rtcNewGeometry(device.handle, GeometryType::FLAT_HERMITE_CURVE) },
        };
        let mut vertex_buffer = Buffer::new(device.clone(), num_verts);
        let mut index_buffer = Buffer::new(device.clone(), num_segments);
        let mut tangent_buffer = Buffer::new(device.clone(), num_verts);
        let mut normal_derivative_buffer = None;
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

            rtcSetGeometryBuffer(
                h,
                BufferType::TANGENT,
                0,
                Format::FLOAT4,
                tangent_buffer.handle,
                0,
                16,
                num_verts,
            );
            tangent_buffer.set_attachment(h, BufferType::TANGENT, 0);

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

                let mut temp_normal_derivative_buffer = Buffer::new(device.clone(), num_verts);
                rtcSetGeometryBuffer(
                    h,
                    BufferType::NORMAL_DERIVATIVE,
                    0,
                    Format::FLOAT3,
                    temp_normal_derivative_buffer.handle,
                    0,
                    12,
                    num_verts,
                );
                temp_normal_derivative_buffer.set_attachment(h, BufferType::NORMAL_DERIVATIVE, 0);
                normal_derivative_buffer = Some(temp_normal_derivative_buffer);
            }
        }
        Arc::new(HermiteCurve {
            device: device,
            handle: h,
            vertex_buffer: vertex_buffer,
            index_buffer: index_buffer,
            tangent_buffer: tangent_buffer,
            normal_derivative_buffer: normal_derivative_buffer,
            normal_buffer: normal_buffer,
        })
    }
}

impl Geometry for HermiteCurve {
    fn handle(&self) -> RTCGeometry {
        self.handle
    }
}

impl Drop for HermiteCurve {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseGeometry(self.handle);
        }
    }
}
