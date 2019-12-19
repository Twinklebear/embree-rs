#![allow(dead_code)]

extern crate cgmath;
extern crate embree_rs;
extern crate support;

use cgmath::{Vector3, Vector4};
use embree_rs::{Device, Geometry, IntersectContext, QuadMesh, Ray, RayHit, Scene, TriangleMesh};
use support::Camera;

fn make_cube<'a>(device: &'a Device) -> Geometry<'a> {
    let mut mesh = TriangleMesh::unanimated(device, 12, 8);
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
    let mut mesh = Geometry::Triangle(mesh);
    mesh.commit();
    mesh
}
fn make_ground_plane<'a>(device: &'a Device) -> Geometry<'a> {
    let mut mesh = QuadMesh::unanimated(device, 1, 4);
    {
        let mut verts = mesh.vertex_buffer.map();
        let mut quads = mesh.index_buffer.map();
        verts[0] = Vector4::new(-10.0, -2.0, -10.0, 0.0);
        verts[1] = Vector4::new(-10.0, -2.0, 10.0, 0.0);
        verts[2] = Vector4::new(10.0, -2.0, 10.0, 0.0);
        verts[3] = Vector4::new(10.0, -2.0, -10.0, 0.0);

        quads[0] = Vector4::new(0, 1, 2, 3);
    }
    let mut mesh = Geometry::Quad(mesh);
    mesh.commit();
    mesh
}

fn main() {
    let mut display = support::Display::new(512, 512, "triangle geometry");
    let device = Device::new();
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
