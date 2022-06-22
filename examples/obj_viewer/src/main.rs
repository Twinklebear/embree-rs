#![allow(dead_code)]

extern crate cgmath;
extern crate embree;
extern crate support;
extern crate tobj;

use std::path::Path;

use cgmath::{InnerSpace, Vector3, Vector4};
use embree::{Device, Geometry, IntersectContext, Ray, RayHit, Scene, TriangleMesh};
use support::{Camera, AABB};

fn main() {
    let mut display = support::Display::new(512, 512, "OBJ Viewer");
    let device = Device::new();

    let args: Vec<_> = std::env::args().collect();
    let (models, _) = tobj::load_obj(&Path::new(&args[1])).unwrap();
    let mut tri_geoms = Vec::new();

    let mut aabb = AABB::default();
    for m in models.iter() {
        let mesh = &m.mesh;
        println!(
            "Mesh has {} triangles and {} verts",
            mesh.indices.len() / 3,
            mesh.positions.len() / 3
        );

        let mut tris =
            TriangleMesh::unanimated(&device, mesh.indices.len() / 3, mesh.positions.len() / 3);
        {
            let mut verts = tris.vertex_buffer.map();
            let mut tris = tris.index_buffer.map();
            for i in 0..mesh.positions.len() / 3 {
                aabb = aabb.union_vec(&Vector3::new(
                    mesh.positions[i * 3],
                    mesh.positions[i * 3 + 1],
                    mesh.positions[i * 3 + 2],
                ));
                verts[i] = Vector4::new(
                    mesh.positions[i * 3],
                    mesh.positions[i * 3 + 1],
                    mesh.positions[i * 3 + 2],
                    0.0,
                );
            }

            for i in 0..mesh.indices.len() / 3 {
                tris[i] = Vector3::new(
                    mesh.indices[i * 3],
                    mesh.indices[i * 3 + 1],
                    mesh.indices[i * 3 + 2],
                );
            }
        }
        let mut tri_geom = Geometry::Triangle(tris);
        tri_geom.commit();
        tri_geoms.push(tri_geom);
    }
    display = display.aabb(aabb);

    let mut scene = Scene::new(&device);
    let mut mesh_ids = Vec::with_capacity(models.len());
    for g in tri_geoms.drain(0..) {
        let id = scene.attach_geometry(g);
        mesh_ids.push(id);
    }
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
            55.0,
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
                    let p = image.get_pixel_mut(i, j);
                    let mesh = &models[mesh_ids[ray_hit.hit.geomID as usize] as usize].mesh;
                    if !mesh.normals.is_empty() {
                        let prim = ray_hit.hit.primID as usize;
                        let tri = [
                            mesh.indices[prim * 3] as usize,
                            mesh.indices[prim * 3 + 1] as usize,
                            mesh.indices[prim * 3 + 2] as usize,
                        ];

                        let na = Vector3::new(
                            mesh.normals[tri[0] * 3],
                            mesh.normals[tri[0] * 3 + 1],
                            mesh.normals[tri[0] * 3 + 2],
                        );

                        let nb = Vector3::new(
                            mesh.normals[tri[1] * 3],
                            mesh.normals[tri[1] * 3 + 1],
                            mesh.normals[tri[1] * 3 + 2],
                        );

                        let nc = Vector3::new(
                            mesh.normals[tri[2] * 3],
                            mesh.normals[tri[2] * 3 + 1],
                            mesh.normals[tri[2] * 3 + 2],
                        );

                        let w = 1.0 - ray_hit.hit.u - ray_hit.hit.v;
                        let mut n = (na * w + nb * ray_hit.hit.u + nc * ray_hit.hit.v).normalize();
                        n = (n + Vector3::new(1.0, 1.0, 1.0)) * 0.5;

                        p[0] = (n.x * 255.0) as u8;
                        p[1] = (n.y * 255.0) as u8;
                        p[2] = (n.z * 255.0) as u8;
                    } else {
                        p[0] = (ray_hit.hit.u * 255.0) as u8;
                        p[1] = (ray_hit.hit.v * 255.0) as u8;
                        p[2] = 0;
                    }
                }
            }
        }
    });
}
