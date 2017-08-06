#![allow(dead_code)]

extern crate embree;
extern crate support;

use std::{ptr, slice, f32, u32};
use support::{Camera, Vec3f};

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

/// Make a triangulated sphere, from the Embree tutorial:
/// https://github.com/embree/embree/blob/master/tutorials/instanced_geometry/instanced_geometry_device.cpp 
fn make_triangulated_sphere(scene: &embree::RTCScene, pos: Vec3f, radius: f32) -> std::os::raw::c_uint {
    let num_phi = 5;
    let num_theta = 2 * num_phi;

    unsafe {
        let geom_id = embree::rtcNewTriangleMesh(*scene, embree::RTCGeometryFlags::RTC_GEOMETRY_STATIC,
                                                 2 * num_theta * (num_phi - 1), num_theta * (num_phi + 1), 1);

        let vbuf = embree::rtcMapBuffer(*scene, geom_id, embree::RTCBufferType::RTC_VERTEX_BUFFER);
        let mut verts: &mut [Vertex] = slice::from_raw_parts_mut(vbuf as *mut Vertex, num_theta * (num_phi + 1));

        let inv_num_phi = 1.0 / (num_phi as f32);
        let inv_num_theta = 1.0 / (num_theta as f32);
        for phi in 0..num_phi + 1 {
            for theta in 0..num_theta {
                let phif = phi as f32 * f32::consts::PI * inv_num_phi;
                let thetaf = theta as f32 * f32::consts::PI * 2.0 * inv_num_theta;

                let v = &mut verts[phi * num_theta + theta];
                v.x = pos.x + radius * f32::sin(phif) * f32::sin(thetaf);
                v.y = pos.y + radius * f32::cos(phif);
                v.z = pos.z + radius * f32::sin(phif) * f32::cos(thetaf);
            }
        }

        let ibuf = embree::rtcMapBuffer(*scene, geom_id, embree::RTCBufferType::RTC_INDEX_BUFFER);
        let mut tris: &mut [Triangle] = slice::from_raw_parts_mut(ibuf as *mut Triangle,
                                                                  2 * num_theta * (num_phi - 1));
        let mut tri = 0;
        for phi in 1..num_phi + 1 {
            for theta in 1..num_theta + 1 {
                let p00 = (phi - 1) * num_theta + theta - 1;
                let p01 = (phi - 1) * num_theta + theta % num_theta;
                let p10 = phi * num_theta + theta - 1;
                let p11 = phi * num_theta + theta % num_theta;

                if phi > 1 {
                    tris[tri].v0 = p10 as i32;
                    tris[tri].v1 = p00 as i32;
                    tris[tri].v2 = p01 as i32;
                    tri += 1;
                }
                if phi < num_phi {
                    tris[tri].v0 = p11 as i32;
                    tris[tri].v1 = p10 as i32;
                    tris[tri].v2 = p01 as i32;
                    tri += 1;
                }
            }
        }

        embree::rtcUnmapBuffer(*scene, geom_id, embree::RTCBufferType::RTC_VERTEX_BUFFER);
        embree::rtcUnmapBuffer(*scene, geom_id, embree::RTCBufferType::RTC_INDEX_BUFFER);

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
            quads[0] = Quad::new(3, 2, 1, 0);
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
        let ground = make_ground_plane(&scene);
        let sphere = make_triangulated_sphere(&scene, Vec3f::new(1.0, 0.0, 0.0), 2.0);

        let instance_colors = vec![Vertex::new(1.0, 1.0, 1.0), Vertex::new(1.0, 0.0, 0.0),
                                   Vertex::new(0.0, 1.0, 0.0), Vertex::new(0.0, 1.0, 0.0),
                                   Vertex::new(0.5, 0.5, 0.5), Vertex::new(0.5, 0.5, 0.5),
                                   Vertex::new(1.0, 1.0, 1.0), Vertex::new(1.0, 1.0, 1.0),
                                   Vertex::new(0.0, 0.0, 1.0), Vertex::new(0.0, 0.0, 1.0),
                                   Vertex::new(1.0, 1.0, 0.0), Vertex::new(1.0, 1.0, 0.0)];

        embree::rtcCommit(scene);

        let light_dir = Vec3f::new(1.0, 1.0, -1.0).normalized();
        display.run(|image, camera_pose| {
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
                        let normal = Vec3f::new(ray.Ng[0], ray.Ng[1], ray.Ng[2]).normalized();
                        let mut illum = 0.3;
                        let shadow_pos = camera.pos + dir * ray.tfar;
                        let mut shadow_ray = embree::RTCRay::new(&[shadow_pos.x, shadow_pos.y, shadow_pos.z],
                                                                 &[light_dir.x, light_dir.y, light_dir.z]);
                        shadow_ray.tnear = 0.001;
                        embree::rtcOccluded(scene, &mut shadow_ray as *mut embree::RTCRay);
                        if shadow_ray.geomID != 0 {
                            illum = support::clamp(illum + f32::max(light_dir.dot(&normal), 0.0), 0.0, 1.0);
                        }

                        if ray.geomID == ground {
                            let mut p = image.get_pixel_mut(i, j);
                            p.data[0] = (255.0 * illum) as u8;
                            p.data[1] = p.data[0];
                            p.data[2] = p.data[0];
                        } else {
                            // Shade the instances as we want
                            let mut p = image.get_pixel_mut(i, j);
                            p.data[0] = (255.0 * illum) as u8;
                            p.data[1] = 0;
                            p.data[2] = 0;
                        }
                    }
                }
            }
        });
        embree::rtcDeleteGeometry(scene, ground);
        embree::rtcDeleteScene(scene);
        embree::rtcDeleteDevice(device);
    }
}

