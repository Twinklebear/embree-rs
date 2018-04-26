#![allow(dead_code)]

extern crate cgmath;
extern crate embree;
extern crate support;

use cgmath::{InnerSpace, Matrix, Matrix4, SquareMatrix, Vector3, Vector4};
use embree::{Device, Geometry, Instance, IntersectContext, QuadMesh, Ray, RayHit, Scene,
             TriangleMesh};
use std::{f32, u32};
use support::Camera;

/// Make a triangulated sphere, from the Embree tutorial:
/// https://github.com/embree/embree/blob/master/tutorials/instanced_geometry/instanced_geometry_device.cpp
fn make_triangulated_sphere<'a>(
    device: &'a Device,
    pos: Vector3<f32>,
    radius: f32,
) -> TriangleMesh<'a> {
    let num_phi = 5;
    let num_theta = 2 * num_phi;
    let mut mesh = TriangleMesh::unanimated(
        device,
        2 * num_theta * (num_phi - 1),
        num_theta * (num_phi + 1),
    );
    {
        let mut verts = mesh.vertex_buffer.map();
        let mut tris = mesh.index_buffer.map();

        let inv_num_phi = 1.0 / (num_phi as f32);
        let inv_num_theta = 1.0 / (num_theta as f32);
        for phi in 0..num_phi + 1 {
            for theta in 0..num_theta {
                let phif = phi as f32 * f32::consts::PI * inv_num_phi;
                let thetaf = theta as f32 * f32::consts::PI * 2.0 * inv_num_theta;

                let v = &mut verts[phi * num_theta + theta];
                v.x = pos.x + radius * f32::sin(phif) * f32::sin(thetaf);
                v.y = pos.y + radius * f32::cos(phif);
                v.z = pos.z + radius * f32::sin(phif) * f32::cos(thetaf);
            }
        }

        let mut tri = 0;
        for phi in 1..num_phi + 1 {
            for theta in 1..num_theta + 1 {
                let p00 = (phi - 1) * num_theta + theta - 1;
                let p01 = (phi - 1) * num_theta + theta % num_theta;
                let p10 = phi * num_theta + theta - 1;
                let p11 = phi * num_theta + theta % num_theta;

                if phi > 1 {
                    tris[tri].x = p10 as u32;
                    tris[tri].y = p01 as u32;
                    tris[tri].z = p00 as u32;
                    tri += 1;
                }
                if phi < num_phi {
                    tris[tri].x = p11 as u32;
                    tris[tri].y = p01 as u32;
                    tris[tri].z = p10 as u32;
                    tri += 1;
                }
            }
        }
    }
    mesh.commit();
    mesh
}
fn make_ground_plane<'a>(device: &'a Device) -> QuadMesh<'a> {
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
    mesh.commit();
    mesh
}
// Animate like the Embree example, returns the (transforms, normal_transforms)
fn animate_instances(time: f32, num_instances: usize) -> (Vec<Matrix4<f32>>, Vec<Matrix4<f32>>) {
    let t0 = 0.7 * time;
    let t1 = 1.5 * time;

    let rot = Matrix4::from_cols(
        Vector4::new(f32::cos(t1), 0.0, f32::sin(t1), 0.0),
        Vector4::new(0.0, 1.0, 0.0, 0.0),
        Vector4::new(-f32::sin(t1), 0.0, f32::cos(t1), 0.0),
        Vector4::new(0.0, 0.0, 0.0, 1.0),
    );

    let mut transforms = Vec::with_capacity(num_instances);
    let mut normal_transforms = Vec::with_capacity(num_instances);
    for i in 0..num_instances {
        let t = t0 + i as f32 * 2.0 * f32::consts::PI / 4.0;
        let trans = Matrix4::<f32>::from_translation(
            2.2 * Vector3::<f32>::new(f32::cos(t), 0.0, f32::sin(t)),
        );
        transforms.push(trans * rot);
        normal_transforms.push(transforms[i].invert().unwrap().transpose());
    }
    (transforms, normal_transforms)
}

fn main() {
    let mut display = support::Display::new(512, 512, "instancing");
    let device = Device::new();

    // Make the scene we'll instance with 4 triangulated spheres.
    let spheres = vec![
        make_triangulated_sphere(&device, Vector3::new(0.0, 0.0, 1.0), 0.5),
        make_triangulated_sphere(&device, Vector3::new(1.0, 0.0, 0.0), 0.5),
        make_triangulated_sphere(&device, Vector3::new(0.0, 0.0, -1.0), 0.5),
        make_triangulated_sphere(&device, Vector3::new(-1.0, 0.0, 0.0), 0.5),
    ];
    let mut instanced_scene = Scene::new(&device);
    for s in &spheres[..] {
        instanced_scene.attach_geometry(s);
    }
    instanced_scene.commit();

    // Make the instances first so their ids will be 0-3 that we can then use
    // directly to index into the instance_colors
    let mut instances = vec![
        Instance::unanimated(&device, &instanced_scene),
        Instance::unanimated(&device, &instanced_scene),
        Instance::unanimated(&device, &instanced_scene),
        Instance::unanimated(&device, &instanced_scene),
    ];
    for i in &mut instances[..] {
        i.commit();
    }

    let instance_colors = vec![
        vec![
            Vector3::new(0.25, 0.0, 0.0),
            Vector3::new(0.5, 0.0, 0.0),
            Vector3::new(0.75, 0.0, 0.0),
            Vector3::new(1.00, 0.0, 0.0),
        ],
        vec![
            Vector3::new(0.0, 0.25, 0.0),
            Vector3::new(0.0, 0.50, 0.0),
            Vector3::new(0.0, 0.75, 0.0),
            Vector3::new(0.0, 1.00, 0.0),
        ],
        vec![
            Vector3::new(0.0, 0.0, 0.25),
            Vector3::new(0.0, 0.0, 0.50),
            Vector3::new(0.0, 0.0, 0.75),
            Vector3::new(0.0, 0.0, 1.00),
        ],
        vec![
            Vector3::new(0.25, 0.25, 0.0),
            Vector3::new(0.50, 0.50, 0.0),
            Vector3::new(0.75, 0.75, 0.0),
            Vector3::new(1.00, 1.00, 0.0),
        ],
    ];

    let ground = make_ground_plane(&device);

    // TODO The commit and set_transform taking &mut self make it not possible
    // to modify them while they're part of a scene. Maybe need to switch to
    // a runtime checked borrowing? E.g. you should only avoid modifying
    // the geometry if the scene is being rendered.
    /*
    let mut scene = Scene::new(&device);
    let ground_id = scene.attach_geometry(&ground);
    for i in &instances[..] {
        scene.attach_geometry(i);
    }
    scene.commit();
    */

    let light_dir = Vector3::new(1.0, 1.0, -1.0).normalize();
    let mut intersection_ctx = IntersectContext::coherent();

    display.run(|image, camera_pose, time| {
        for p in image.iter_mut() {
            *p = 0;
        }
        let (transforms, normal_transforms) = animate_instances(time, instances.len());
        for (i, t) in transforms.iter().enumerate() {
            instances[i].set_transform(&t);
            instances[i].commit();
        }
        // TODO The commit and set_transform taking &mut self make it not possible
        // to modify them while they're part of a scene. Maybe need to switch to
        // a runtime checked borrowing? E.g. you should only avoid modifying
        // the geometry if the scene is being rendered.
        let mut scene = Scene::new(&device);
        // TODO VERY BAD
        for i in &instances[..] {
            scene.attach_geometry(i);
        }
        // TODO VERY BAD
        let ground_id = scene.attach_geometry(&ground);
        // TODO VERY BAD
        scene.commit();

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
                let mut ray_hit = RayHit::new(Ray::new(camera.pos, dir));
                scene.intersect(&mut intersection_ctx, &mut ray_hit);

                if ray_hit.hit.hit() {
                    // Transform the normals of the instances into world space with the normal_transforms
                    let hit = &ray_hit.hit;
                    let geom_id = hit.geomID;
                    let inst_id = hit.instID[0];
                    let mut normal = Vector3::new(hit.Ng_x, hit.Ng_y, hit.Ng_z).normalize();
                    if inst_id != u32::MAX {
                        let v = normal_transforms[inst_id as usize]
                            * Vector4::new(normal.x, normal.y, normal.z, 0.0);
                        normal = Vector3::new(v.x, v.y, v.z).normalize()
                    }
                    let mut illum = 0.3;
                    let shadow_pos = camera.pos + dir * ray_hit.ray.tfar;
                    let mut shadow_ray = Ray::segment(shadow_pos, light_dir, 0.001, f32::INFINITY);
                    scene.occluded(&mut intersection_ctx, &mut shadow_ray);

                    if shadow_ray.tfar >= 0.0 {
                        illum =
                            support::clamp(illum + f32::max(light_dir.dot(normal), 0.0), 0.0, 1.0);
                    }

                    let mut p = image.get_pixel_mut(i, j);
                    if inst_id == u32::MAX && geom_id == ground_id {
                        p.data[0] = (255.0 * illum) as u8;
                        p.data[1] = p.data[0];
                        p.data[2] = p.data[0];
                    } else {
                        // Shade the instances using their color
                        let color = &instance_colors[inst_id as usize][geom_id as usize];
                        p.data[0] = (255.0 * illum * color.x) as u8;
                        p.data[1] = (255.0 * illum * color.y) as u8;
                        p.data[2] = (255.0 * illum * color.z) as u8;
                    }
                }
            }
        }
    });
}
