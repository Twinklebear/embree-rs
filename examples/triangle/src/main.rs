#![allow(dead_code)]

extern crate cgmath;
extern crate embree;
extern crate support;

use cgmath::{Vector3, Vector4};
use embree::{Device, Geometry, IntersectContext, RayHitN, RayN, Scene, TriangleMesh};
use std::sync::Arc;

fn main() {
    let display = support::Display::new(512, 512, "triangle");

    let device = Device::new();

    // Make a triangle
    let mut triangle = TriangleMesh::unanimated(device.clone(), 1, 3);
    {
        // TODO: API ergonomics are also pretty rough here w/ all the Arc::get_mut etc
        let tri_mut = Arc::get_mut(&mut triangle).unwrap();
        {
            let mut verts = tri_mut.vertex_buffer.map();
            let mut tris = tri_mut.index_buffer.map();
            verts[0] = [-1.0, 0.0, 0.0, 0.0];
            verts[1] = [0.0, 1.0, 0.0, 0.0];
            verts[2] = [1.0, 0.0, 0.0, 0.0];

            tris[0] = [0, 1, 2];
        }

        tri_mut.commit();
    }

    let mut scene = Scene::new(device.clone());
    {
        let scene_mut = Arc::get_mut(&mut scene).unwrap();
        scene_mut.attach_geometry(triangle);
        scene_mut.commit();
    }

    support::display::run(display, move |image, _, _| {
        let mut intersection_ctx = IntersectContext::coherent();

        let img_dims = image.dimensions();
        // Render the scene
        for j in 0..img_dims.1 {
            let y = -(j as f32 + 0.5) / img_dims.1 as f32 + 0.5;

            // Try out streams of scanlines across x
            let mut rays = RayN::new(img_dims.0 as usize);
            for (i, mut ray) in rays.iter_mut().enumerate() {
                let x = (i as f32 + 0.5) / img_dims.0 as f32 - 0.5;
                let dir_len = f32::sqrt(x * x + y * y + 1.0);
                ray.set_origin([0.0, 0.5, 2.0]);
                ray.set_dir([x / dir_len, y / dir_len, -1.0 / dir_len]);
            }

            let mut ray_hit = RayHitN::new(rays);
            scene.intersect_stream_soa(&mut intersection_ctx, &mut ray_hit);
            for (i, hit) in ray_hit.hit.iter().enumerate().filter(|(_i, h)| h.hit()) {
                let p = image.get_pixel_mut(i as u32, j);
                let uv = hit.uv();
                p[0] = (uv.0 * 255.0) as u8;
                p[1] = (uv.1 * 255.0) as u8;
                p[2] = 0;
            }
        }
    });
}
