#![allow(dead_code)]

extern crate embree;
extern crate support;

use embree::{BufferUsage, Device, Geometry, IntersectContext, RayHitN, RayN, TriangleMesh};
use std::sync::Arc;

fn main() {
    let display = support::Display::new(512, 512, "triangle");

    let device = Device::new().unwrap();

    device.set_error_function(|error, message| {
        println!("Embree error {}: {}", error, message);
    });

    // Make a triangle
    let mut triangle = TriangleMesh::unanimated(&device, 1, 3);
    triangle
        .get_buffer(BufferUsage::VERTEX, 0)
        .unwrap()
        .view_mut::<[f32; 4]>()
        .unwrap()
        .copy_from_slice(&[
            [-1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
        ]);
    triangle
        .get_buffer(BufferUsage::INDEX, 0)
        .unwrap()
        .view_mut::<[u32; 3]>()
        .unwrap()
        .copy_from_slice(&[[0, 1, 2]]);
    triangle.commit();

    let triangle = Arc::new(triangle);
    let mut scene = device.create_scene().unwrap();
    scene.attach_geometry(triangle);
    scene.commit();

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
