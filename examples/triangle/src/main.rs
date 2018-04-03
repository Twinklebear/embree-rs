#![allow(dead_code)]

extern crate embree;
extern crate support;
extern crate cgmath;

use std::{f32, u32};
use cgmath::{Vector3, Vector4};

fn main() {
    let mut display = support::Display::new(512, 512, "triangle");
    let device = embree::Device::new();
    let mut scene = embree::Scene::new(&device);

    // Make a triangle
    let mut triangle = embree::TriangleMesh::unanimated(&device, 1, 3);
    {
        let mut verts = triangle.vertex_buffer.map();
        let mut tris = triangle.index_buffer.map();
        verts[0] = Vector4::new(-1.0, 0.0, 0.0, 0.0);
        verts[1] = Vector4::new(0.0, 1.0, 0.0, 0.0);
        verts[2] = Vector4::new(1.0, 0.0, 0.0, 0.0);

        tris[0] = Vector3::new(0, 1, 2);
    }
    triangle.commit();
    scene.attach_geometry(&triangle);
    scene.commit();

    let mut intersection_ctx = embree::IntersectContext::coherent();

    display.run(|image, _, _| {
        let img_dims = image.dimensions();
        // Render the scene
        for j in 0..img_dims.1 {
            let y = -(j as f32 + 0.5) / img_dims.1 as f32 + 0.5;
            for i in 0..img_dims.0 {
                let x = (i as f32 + 0.5) / img_dims.0 as f32 - 0.5;
                let dir_len = f32::sqrt(x * x + y * y + 1.0);
                let ray = embree::Ray::new(Vector3::new(0.0, 0.5, 2.0),
                                            Vector3::new(x / dir_len, y / dir_len, -1.0 / dir_len));
                let mut ray_hit = embree::RayHit::new(ray);
                scene.intersect(&mut intersection_ctx, &mut ray_hit);
                if ray_hit.hit.geomID != u32::MAX {
                    let p = image.get_pixel_mut(i, j);
                    p.data[0] = (ray_hit.hit.u * 255.0) as u8;
                    p.data[1] = (ray_hit.hit.v * 255.0) as u8;
                    p.data[2] = 0;
                }
            }
        }
    });
}

