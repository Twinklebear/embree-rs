#![allow(dead_code)]

extern crate embree;
extern crate support;

use std::{ptr, slice, f32, u32};

#[repr(C)]
struct Vertex {
    x: f32,
    y: f32,
    z: f32,
    a: f32,
}
#[repr(C)]
struct Triangle {
    v0: i32,
    v1: i32,
    v2: i32,
}

fn main() {
    let mut display = support::Display::new(512, 512, "triangle");
    unsafe {
        let device = embree::rtcNewDevice(ptr::null());
        let scene = embree::rtcDeviceNewScene(device, embree::RTCSceneFlags::RTC_SCENE_STATIC,
                                              embree::RTCAlgorithmFlags::RTC_INTERSECT1);

        // Make a triangle
        let geom_id = embree::rtcNewTriangleMesh2(scene, embree::RTCGeometryFlags::RTC_GEOMETRY_STATIC,
                                                 1, 3, 1, 1);
        {
            let buf = embree::rtcMapBuffer(scene, geom_id, embree::RTCBufferType::RTC_VERTEX_BUFFER);
            let mut verts: &mut [Vertex] = slice::from_raw_parts_mut(buf as *mut Vertex, 3);
            verts[0] = Vertex { x: -1.0, y: 0.0, z: 0.0, a: 0.0 };
            verts[1] = Vertex { x: 0.0, y: 1.0, z: 0.0, a: 0.0 };
            verts[2] = Vertex { x: 1.0, y: 0.0, z: 0.0, a: 0.0 };
            embree::rtcUnmapBuffer(scene, geom_id, embree::RTCBufferType::RTC_VERTEX_BUFFER);
        }
        {
            let buf = embree::rtcMapBuffer(scene, geom_id, embree::RTCBufferType::RTC_INDEX_BUFFER);
            let mut tris: &mut [Triangle] = slice::from_raw_parts_mut(buf as *mut Triangle, 1);
            tris[0] = Triangle { v0: 0, v1: 1, v2: 2 };
            embree::rtcUnmapBuffer(scene, geom_id, embree::RTCBufferType::RTC_INDEX_BUFFER);
        }
        embree::rtcCommit(scene);

        display.run(|image| {
            let img_dims = image.dimensions();
            // Render the scene
            for j in 0..img_dims.1 {
                let y = -(j as f32 + 0.5) / img_dims.1 as f32 + 0.5;
                for i in 0..img_dims.0 {
                    let x = (i as f32 + 0.5) / img_dims.0 as f32 - 0.5;
                    let dir_len = f32::sqrt(x * x + y * y + 1.0);
                    let mut ray = embree::RTCRay::new(&[0.0, 0.5, 2.0],
                                                      &[x / dir_len, y / dir_len, -1.0 / dir_len]);
                    embree::rtcIntersect(scene, &mut ray as *mut embree::RTCRay);
                    if ray.geomID != u32::MAX {
                        let mut p = image.get_pixel_mut(i, j);
                        p.data[0] = (ray.u * 255.0) as u8;
                        p.data[1] = (ray.v * 255.0) as u8;
                        p.data[2] = 0;
                    }
                }
            }
        });
        embree::rtcDeleteScene(scene);
        embree::rtcDeleteDevice(device);
    }
}

