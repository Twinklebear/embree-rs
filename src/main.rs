#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

extern crate image;

use std::{ptr, slice, f32, u32, iter};

mod embree;

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
    unsafe {
        let device = embree::rtcNewDevice(ptr::null());
        let scene = embree::rtcDeviceNewScene(device, embree::RTCSceneFlags::RTC_SCENE_STATIC,
                                              embree::RTCAlgorithmFlags::RTC_INTERSECT1);

        // Make a triangle
        let geomID = embree::rtcNewTriangleMesh2(scene, embree::RTCGeometryFlags::RTC_GEOMETRY_STATIC,
                                                 1, 3, 1, 1);
        {
            let mut buf = embree::rtcMapBuffer(scene, geomID, embree::RTCBufferType::RTC_VERTEX_BUFFER);
            let mut verts: &mut [Vertex] = slice::from_raw_parts_mut(buf as *mut Vertex, 3);
            verts[0] = Vertex { x: -1.0, y: 0.0, z: 0.0, a: 0.0 };
            verts[1] = Vertex { x: 0.0, y: 1.0, z: 0.0, a: 0.0 };
            verts[2] = Vertex { x: 1.0, y: 0.0, z: 0.0, a: 0.0 };
            embree::rtcUnmapBuffer(scene, geomID, embree::RTCBufferType::RTC_VERTEX_BUFFER);
        }
        {
            let mut buf = embree::rtcMapBuffer(scene, geomID, embree::RTCBufferType::RTC_INDEX_BUFFER);
            let mut tris: &mut [Triangle] = slice::from_raw_parts_mut(buf as *mut Triangle, 1);
            tris[0] = Triangle { v0: 0, v1: 1, v2: 2 };
            embree::rtcUnmapBuffer(scene, geomID, embree::RTCBufferType::RTC_INDEX_BUFFER);
        }

        embree::rtcCommit(scene);

        let img_dims = 256;
        let mut image: Vec<_> = iter::repeat(0u8).take(img_dims * img_dims).collect();
        // Render the scene
        for j in 0..img_dims {
            let y = -(j as f32 + 0.5) / img_dims as f32 + 0.5;
            for i in 0..img_dims {
                let x = (i as f32 + 0.5) / img_dims as f32 - 0.5;
                let dirLength = f32::sqrt(x * x + y * y + 1.0);
                let mut ray = embree::RTCRay {
                    org: [0.0, 0.0, 2.0],
                    align0: 0.0,
                    dir: [x / dirLength, y / dirLength, -1.0 / dirLength],
                    align1: 0.0,
                    tnear: 0.0,
                    tfar: f32::INFINITY,
                    time: 0.0,
                    mask: u32::MAX,
                    Ng: [0.0; 3],
                    align2: 0.0,
                    u: 0.0,
                    v: 0.0,
                    geomID: u32::MAX,
                    primID: u32::MAX,
                    instID: u32::MAX,
                    __bindgen_padding_0: [0; 3],
                };
                embree::rtcIntersect(scene, &mut ray as *mut embree::RTCRay);
                if ray.geomID != u32::MAX {
                    image[j * img_dims + i] = 255;
                }
            }
        }
        embree::rtcDeleteScene(scene);
        embree::rtcDeleteDevice(device);

        match image::save_buffer("result.png", &image[..], img_dims as u32, img_dims as u32, image::Gray(8)) {
            Ok(_) => println!("Result saved to result.png"),
            Err(e) => panic!("Error saving image: {}", e),
        }
    }
}

