#![allow(dead_code)]

extern crate embree;
extern crate support;

use std::{ptr, slice, f32, u32};
use support::Camera;

// TODO: Roll these types up into the Embree-rs library
#[repr(C)]
struct Vertex {
    x: f32,
    y: f32,
    z: f32,
    a: f32,
}
impl Vertex {
    pub fn new(x: f32, y: f32, z: f32) -> Vertex {
        Vertex { x: x, y: y, z: z, a: 0.0 }
    }
}
#[repr(C)]
struct Triangle {
    v0: i32,
    v1: i32,
    v2: i32,
}
impl Triangle {
    pub fn new(v0: i32, v1: i32, v2: i32) -> Triangle {
        Triangle { v0: v0, v1: v1, v2: v2 }
    }
}
#[repr(C)]
struct Quad {
    v0: i32,
    v1: i32,
    v2: i32,
    v3: i32,
}
impl Quad {
    pub fn new(v0: i32, v1: i32, v2: i32, v3: i32) -> Quad {
        Quad { v0: v0, v1: v1, v2: v2, v3: v3 }
    }
}

fn make_cube(scene: &embree::RTCScene) -> std::os::raw::c_uint {
    unsafe {
        let geom_id = embree::rtcNewTriangleMesh(*scene, embree::RTCGeometryFlags::RTC_GEOMETRY_STATIC,
                                                 12, 8, 1);
        {
            let buf = embree::rtcMapBuffer(*scene, geom_id, embree::RTCBufferType::RTC_VERTEX_BUFFER);
            let mut verts: &mut [Vertex] = slice::from_raw_parts_mut(buf as *mut Vertex, 8);
            verts[0] = Vertex::new(-1.0, -1.0, -1.0);
            verts[1] = Vertex::new(-1.0, -1.0, 1.0);
            verts[2] = Vertex::new(-1.0, 1.0, -1.0);
            verts[3] = Vertex::new(-1.0, 1.0, 1.0);
            verts[4] = Vertex::new(1.0, -1.0, -1.0);
            verts[5] = Vertex::new(1.0, -1.0, 1.0);
            verts[6] = Vertex::new(1.0, 1.0, -1.0);
            verts[7] = Vertex::new(1.0, 1.0, 1.0);
            embree::rtcUnmapBuffer(*scene, geom_id, embree::RTCBufferType::RTC_VERTEX_BUFFER);
        }
        {
            let buf = embree::rtcMapBuffer(*scene, geom_id, embree::RTCBufferType::RTC_INDEX_BUFFER);
            let mut tris: &mut [Triangle] = slice::from_raw_parts_mut(buf as *mut Triangle, 12);

            // left side
            tris[0] = Triangle::new(0, 2, 1);
            tris[1] = Triangle::new(1, 2, 3);

            // right side
            tris[2] = Triangle::new(4, 5, 6);
            tris[3] = Triangle::new(5, 7, 6);

            // bottom side
            tris[4] = Triangle::new(0, 1, 4);
            tris[5] = Triangle::new(1, 5, 4);

            // top side
            tris[6] = Triangle::new(2, 6, 3);
            tris[7] = Triangle::new(3, 6, 7);

            // front side
            tris[8] = Triangle::new(0, 4, 2);
            tris[9] = Triangle::new(2, 4, 6);

            // back side
            tris[10] = Triangle::new(1, 3, 5);
            tris[11] = Triangle::new(3, 7, 5);

            embree::rtcUnmapBuffer(*scene, geom_id, embree::RTCBufferType::RTC_INDEX_BUFFER);
        }
        geom_id
    }
}
fn make_ground_plane(scene: &embree::RTCScene) -> std::os::raw::c_uint {
    unsafe {
        let geom_id = embree::rtcNewQuadMesh(*scene, embree::RTCGeometryFlags::RTC_GEOMETRY_STATIC,
                                             1, 4, 1);
        {
            let buf = embree::rtcMapBuffer(*scene, geom_id, embree::RTCBufferType::RTC_VERTEX_BUFFER);
            let mut verts: &mut [Vertex] = slice::from_raw_parts_mut(buf as *mut Vertex, 4);
            verts[0] = Vertex::new(-10.0, -2.0, -10.0);
            verts[1] = Vertex::new(-10.0, -2.0, 10.0);
            verts[2] = Vertex::new(10.0, -2.0, 10.0);
            verts[3] = Vertex::new(10.0, -2.0, -10.0);
            embree::rtcUnmapBuffer(*scene, geom_id, embree::RTCBufferType::RTC_VERTEX_BUFFER);
        }
        {
            let buf = embree::rtcMapBuffer(*scene, geom_id, embree::RTCBufferType::RTC_INDEX_BUFFER);
            let mut quads: &mut [Quad] = slice::from_raw_parts_mut(buf as *mut Quad, 1);
            quads[0] = Quad::new(0, 1, 2, 3);
            embree::rtcUnmapBuffer(*scene, geom_id, embree::RTCBufferType::RTC_INDEX_BUFFER);
        }
        geom_id
    }
}

fn main() {
    let mut display = support::Display::new(512, 512, "triangle geometry");
    unsafe {
        let device = embree::rtcNewDevice(ptr::null());
        let scene = embree::rtcDeviceNewScene(device, embree::RTCSceneFlags::RTC_SCENE_STATIC,
                                              embree::RTCAlgorithmFlags::RTC_INTERSECT1);
        let cube = make_cube(&scene);
        let ground = make_ground_plane(&scene);

        let face_colors = vec![Vertex::new(1.0, 0.0, 0.0), Vertex::new(1.0, 0.0, 0.0),
                                    Vertex::new(0.0, 1.0, 0.0), Vertex::new(0.0, 1.0, 0.0),
                                    Vertex::new(0.5, 0.5, 0.5), Vertex::new(0.5, 0.5, 0.5),
                                    Vertex::new(1.0, 1.0, 1.0), Vertex::new(1.0, 1.0, 1.0),
                                    Vertex::new(0.0, 0.0, 1.0), Vertex::new(0.0, 0.0, 1.0),
                                    Vertex::new(1.0, 1.0, 0.0), Vertex::new(1.0, 1.0, 0.0)];

        embree::rtcCommit(scene);

        display.run(|image, camera_pose, _| {
            for p in image.iter_mut() {
                *p = 0;
            }
            let img_dims = image.dimensions();
            let camera = Camera::look_dir(camera_pose.pos, camera_pose.dir,
                                         camera_pose.up, 75.0, img_dims);
            // Render the scene
            for j in 0..img_dims.1 {
                for i in 0..img_dims.0 {
                    let dir = camera.ray_dir((i as f32 + 0.5, j as f32 + 0.5));
                    let mut ray = embree::RTCRay::new(&[camera.pos.x, camera.pos.y, camera.pos.z],
                                                      &[dir.x, dir.y, dir.z]);
                    embree::rtcIntersect(scene, &mut ray as *mut embree::RTCRay);
                    if ray.geomID != u32::MAX {
                        let color = &face_colors[ray.primID as usize];
                        let mut p = image.get_pixel_mut(i, j);
                        p.data[0] = (color.x * 255.0) as u8;
                        p.data[1] = (color.y * 255.0) as u8;
                        p.data[2] = (color.z * 255.0) as u8;
                    }
                }
            }
        });
        embree::rtcDeleteGeometry(scene, cube);
        embree::rtcDeleteGeometry(scene, ground);
        embree::rtcDeleteScene(scene);
        embree::rtcDeleteDevice(device);
    }
}

