#![allow(dead_code)]

extern crate embree;
extern crate support;
extern crate cgmath;

use std::{f32, u32};
use support::Camera;
use cgmath::{Vector3, Vector4};

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

fn make_cube<'a>(scene: &'a embree::Scene) -> embree::TriangleMesh<'a> {
    let mut mesh = embree::TriangleMesh::unanimated(scene, embree::GeometryFlags::STATIC,
                                                12, 8);
    {
        let mut verts = mesh.vertex_buffer.map();
        let mut tris = mesh.index_buffer.map();

        verts[0] = Vector4::new(-1.0, -1.0, -1.0, 0.0);
        verts[1] = Vector4::new(-1.0, -1.0, 1.0, 0.0);
        verts[2] = Vector4::new(-1.0, 1.0, -1.0, 0.0);
        verts[3] = Vector4::new(-1.0, 1.0, 1.0, 0.0);
        verts[4] = Vector4::new(1.0, -1.0, -1.0, 0.0);
        verts[5] = Vector4::new(1.0, -1.0, 1.0, 0.0);
        verts[6] = Vector4::new(1.0, 1.0, -1.0, 0.0);
        verts[7] = Vector4::new(1.0, 1.0, 1.0, 0.0);

        // left side
        tris[0] = Vector3::new(0, 2, 1);
        tris[1] = Vector3::new(1, 2, 3);

        // right side
        tris[2] = Vector3::new(4, 5, 6);
        tris[3] = Vector3::new(5, 7, 6);

        // bottom side
        tris[4] = Vector3::new(0, 1, 4);
        tris[5] = Vector3::new(1, 5, 4);

        // top side
        tris[6] = Vector3::new(2, 6, 3);
        tris[7] = Vector3::new(3, 6, 7);

        // front side
        tris[8] = Vector3::new(0, 4, 2);
        tris[9] = Vector3::new(2, 4, 6);

        // back side
        tris[10] = Vector3::new(1, 3, 5);
        tris[11] = Vector3::new(3, 7, 5);
    }
    mesh
}
fn make_ground_plane<'a>(scene: &'a embree::Scene) -> embree::QuadMesh<'a> {
    let mut mesh = embree::QuadMesh::unanimated(scene, embree::GeometryFlags::STATIC,
                                                1, 4);
    {
        let mut verts = mesh.vertex_buffer.map();
        let mut quads = mesh.index_buffer.map();
        verts[0] = Vector4::new(-10.0, -2.0, -10.0, 0.0);
        verts[1] = Vector4::new(-10.0, -2.0, 10.0, 0.0);
        verts[2] = Vector4::new(10.0, -2.0, 10.0, 0.0);
        verts[3] = Vector4::new(10.0, -2.0, -10.0, 0.0);

        quads[0] = Vector4::<i32>::new(0, 1, 2, 3);
    }
    mesh
}

fn main() {
    let mut display = support::Display::new(512, 512, "triangle geometry");
    let device = embree::Device::new();
    let scene = embree::Scene::new(&device, embree::SceneFlags::SCENE_STATIC,
                                   embree::AlgorithmFlags::INTERSECT1);
    let cube = make_cube(&scene);
    let ground = make_ground_plane(&scene);

    let face_colors = vec![Vector3::new(1.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0),
                            Vector3::new(0.0, 1.0, 0.0), Vector3::new(0.0, 1.0, 0.0),
                            Vector3::new(0.5, 0.5, 0.5), Vector3::new(0.5, 0.5, 0.5),
                            Vector3::new(1.0, 1.0, 1.0), Vector3::new(1.0, 1.0, 1.0),
                            Vector3::new(0.0, 0.0, 1.0), Vector3::new(0.0, 0.0, 1.0),
                            Vector3::new(1.0, 1.0, 0.0), Vector3::new(1.0, 1.0, 0.0)];

    scene.commit();

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
                let mut ray = embree::Ray::new(&camera.pos, &dir);
                scene.intersect(&mut ray);
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
}

