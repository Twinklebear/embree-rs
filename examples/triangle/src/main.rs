#![allow(dead_code)]

extern crate cgmath;
extern crate sol;
extern crate support;

use cgmath::{Vector3, Vector4};
use sol::{Device, Geometry, IntersectContext, Ray4, RayHit4, Scene, TriangleMesh};

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
            for i in 0..img_dims.0 / 4 {
                let mut origs = [Vector3::new(0.0, 0.0, 0.0); 4];
                let mut dirs = [Vector3::new(0.0, 0.0, 0.0); 4];
                for k in 0..4 {
                    let x = ((i * 4 + k) as f32 + 0.5) / img_dims.0 as f32 - 0.5;
                    let dir_len = f32::sqrt(x * x + y * y + 1.0);
                    origs[k as usize] = Vector3::new(0.0, 0.5, 2.0);
                    dirs[k as usize] = Vector3::new(x / dir_len, y / dir_len, -1.0 / dir_len);
                }
                let rays = Ray4::new(origs, dirs);
                let mut ray_hit = RayHit4::new(rays);
                let valid = [-1; 4];
                rtscene.intersect4(&mut intersection_ctx, &mut ray_hit, &valid);
                for (k, hit) in ray_hit.hit.iter().enumerate().filter(|(_k, h)| h.hit()) {
                    let p = image.get_pixel_mut(i * 4 + k as u32, j);
                    let uv = hit.uv();
                    p.data[0] = (uv.0 * 255.0) as u8;
                    p.data[1] = (uv.1 * 255.0) as u8;
                    p.data[2] = 0;
                }
            }
        }
    });
}

