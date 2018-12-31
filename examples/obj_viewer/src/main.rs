#![allow(dead_code)]

extern crate cgmath;
extern crate sol;
extern crate support;
extern crate tobj;

use std::path::Path;

use cgmath::{Vector3, Vector4};
use sol::{Device, Geometry, IntersectContext, Ray, RayHit, Scene, TriangleMesh};
use support::Camera;

fn main() {
    let mut display = support::Display::new(512, 512, "OBJ Viewer");
    let device = Device::new();

    let args: Vec<_> = std::env::args().collect();
    let (models, _) = tobj::load_obj(&Path::new(&args[1])).unwrap();
    let mesh = &models[0].mesh;

    println!("Mesh has {} triangles and {} verts",
             mesh.indices.len() / 3, mesh.positions.len() / 3);

    // Make a triangle
    let mut tris = TriangleMesh::unanimated(&device,
                                            mesh.indices.len() / 3,
                                            mesh.positions.len() / 3);
    {
        let mut verts = tris.vertex_buffer.map();
        let mut tris = tris.index_buffer.map();
        for i in 0..mesh.positions.len() / 3 { 
            verts[i] = Vector4::new(mesh.positions[i * 3],
                                    mesh.positions[i * 3 + 1],
                                    mesh.positions[i * 3 + 2],
                                    0.0);
        }

        for i in 0..mesh.indices.len() / 3 { 
            tris[i] = Vector3::new(mesh.indices[i * 3],
                                   mesh.indices[i * 3 + 1],
                                   mesh.indices[i * 3 + 2]);
        }
    }
    let mut tri_geom = Geometry::Triangle(tris);
    tri_geom.commit();

    let mut scene = Scene::new(&device);
    scene.attach_geometry(tri_geom);
    let rtscene = scene.commit();

    let mut intersection_ctx = IntersectContext::coherent();

    display.run(|image, camera_pose, _| {
        for p in image.iter_mut() {
            *p = 0;
        }
        let img_dims = image.dimensions();
        let camera = Camera::look_dir(
            camera_pose.pos,
            camera_pose.dir,
            camera_pose.up,
            75.0,
            img_dims,
        );
        // Render the scene
        for j in 0..img_dims.1 {
            for i in 0..img_dims.0 {
                let dir = camera.ray_dir((i as f32 + 0.5, j as f32 + 0.5));
                let ray = Ray::new(camera.pos, dir);
                let mut ray_hit = RayHit::new(ray);
                rtscene.intersect(&mut intersection_ctx, &mut ray_hit);
                if ray_hit.hit.hit() {
                    let mut p = image.get_pixel_mut(i, j);
                    p.data[0] = (ray_hit.hit.u * 255.0) as u8;
                    p.data[1] = (ray_hit.hit.v * 255.0) as u8;
                    p.data[2] = 0;
                }
            }
        }
    });
}

