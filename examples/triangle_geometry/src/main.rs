#![allow(dead_code)]

extern crate cgmath;
extern crate embree;
extern crate support;

use cgmath::{Vector3, Vector4};
use embree::{
    BufferUsage, Device, Geometry, GeometryVertexAttribute, IntersectContext, QuadMesh, Ray,
    RayHit, Scene, TriangleMesh,
};
use std::sync::Arc;
use support::Camera;

fn make_cube(device: &Device) -> Arc<dyn Geometry> {
    let mut mesh = TriangleMesh::unanimated(device, 12, 8);
    {
        mesh.get_buffer(BufferUsage::VERTEX, 0)
            .unwrap()
            .view_mut::<[f32; 3]>()
            .unwrap()
            .copy_from_slice(&[
                [-1.0, -1.0, -1.0],
                [-1.0, -1.0, 1.0],
                [-1.0, 1.0, -1.0],
                [-1.0, 1.0, 1.0],
                [1.0, -1.0, -1.0],
                [1.0, -1.0, 1.0],
                [1.0, 1.0, -1.0],
                [1.0, 1.0, 1.0],
            ]);

        mesh.get_buffer(BufferUsage::INDEX, 0)
            .unwrap()
            .view_mut::<[u32; 3]>()
            .unwrap()
            .copy_from_slice(&[
                // left side
                [0, 2, 1],
                [1, 2, 3],
                // right side
                [4, 5, 6],
                [5, 7, 6],
                // bottom side
                [0, 1, 4],
                [1, 5, 4],
                // top side
                [2, 6, 3],
                [3, 6, 7],
                // front side
                [0, 4, 2],
                [2, 4, 6],
                // back side
                [1, 3, 5],
                [3, 7, 5],
            ]);
    }
    let mut mesh = Geometry::Triangle(mesh);
    mesh.commit();
    Arc::new(mesh)
}

fn make_ground_plane(device: &Device) -> Arc<dyn Geometry> {
    let mut mesh = QuadMesh::unanimated(device, 1, 4);
    {
        mesh.get_buffer(BufferUsage::VERTEX, 0)
            .unwrap()
            .view_mut::<[f32; 4]>()
            .unwrap()
            .copy_from_slice(&[
                [-10.0, -2.0, -10.0, 0.0],
                [-10.0, -2.0, 10.0, 0.0],
                [10.0, -2.0, 10.0, 0.0],
                [10.0, -2.0, -10.0, 0.0],
            ]);
        mesh.get_buffer(BufferUsage::INDEX, 0)
            .unwrap()
            .view_mut::<[u32; 4]>()
            .unwrap()
            .copy_from_slice(&[[0, 1, 2, 3]]);
    }
    mesh.set_vertex_attribute_count(1);
    //    mesh.set_shared
    mesh.commit();
    Arc::new(mesh)
}

fn main() {
    let mut display = support::Display::new(512, 512, "triangle geometry");
    let device = Device::new().unwrap();
    let cube = make_cube(&device);
    let ground = make_ground_plane(&device);

    // TODO: Support for Embree3's new vertex attributes
    let face_colors = vec![
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(1.0, 0.0, 1.0),
        Vector3::new(1.0, 0.0, 1.0),
        Vector3::new(1.0, 1.0, 1.0),
        Vector3::new(1.0, 1.0, 1.0),
        Vector3::new(0.0, 0.0, 1.0),
        Vector3::new(0.0, 0.0, 1.0),
        Vector3::new(1.0, 1.0, 0.0),
        Vector3::new(1.0, 1.0, 0.0),
    ];

    let mut scene = Scene::new(&device);
    scene.attach_geometry(cube);
    let ground_id = scene.attach_geometry(ground);
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
                    let color = if ray_hit.hit.geomID == ground_id {
                        Vector3::new(0.6, 0.6, 0.6)
                    } else {
                        face_colors[ray_hit.hit.primID as usize]
                    };
                    p[0] = (color.x * 255.0) as u8;
                    p[1] = (color.y * 255.0) as u8;
                    p[2] = (color.z * 255.0) as u8;
                }
            }
        }
    });
}
