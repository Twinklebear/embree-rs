#![allow(dead_code)]

extern crate cgmath;
extern crate embree_rs;
extern crate support;

use cgmath::{Vector3, Vector4};
use embree_rs::{Device, Geometry, IntersectContext, RayN, RayHitN, Scene, TriangleMesh};

fn main() {
    let mut display = support::Display::new(512, 512, "triangle");
    let device = Device::new();

    // Make a triangle
    let mut triangle = TriangleMesh::unanimated(&device, 1, 3);
    {
        let mut verts = triangle.vertex_buffer.map();
        let mut tris = triangle.index_buffer.map();
        verts[0] = Vector4::new(-1.0, 0.0, 0.0, 0.0);
        verts[1] = Vector4::new(0.0, 1.0, 0.0, 0.0);
        verts[2] = Vector4::new(1.0, 0.0, 0.0, 0.0);

        tris[0] = Vector3::new(0, 1, 2);
    }
    let mut tri_geom = Geometry::Triangle(triangle);
    tri_geom.commit();

    let mut scene = Scene::new(&device);
    scene.attach_geometry(tri_geom);
    let rtscene = scene.commit();

    let mut intersection_ctx = IntersectContext::coherent();

    display.run(|image, _, _| {
        let img_dims = image.dimensions();
        // Render the scene
        for j in 0..img_dims.1 {
            let y = -(j as f32 + 0.5) / img_dims.1 as f32 + 0.5;

            // Try out streams of scanlines across x
            let mut rays = RayN::new(img_dims.0 as usize);
            for (i, mut ray) in rays.iter_mut().enumerate() {
                let x = (i as f32 + 0.5) / img_dims.0 as f32 - 0.5;
                let dir_len = f32::sqrt(x * x + y * y + 1.0);
                ray.set_origin(Vector3::new(0.0, 0.5, 2.0));
                ray.set_dir(Vector3::new(x / dir_len, y / dir_len, -1.0 / dir_len));
            }

            let mut ray_hit = RayHitN::new(rays);
            rtscene.intersect_stream_soa(&mut intersection_ctx, &mut ray_hit);
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

