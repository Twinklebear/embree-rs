#![allow(dead_code)]

extern crate embree;
extern crate support;
extern crate cgmath;

use std::{ptr, slice, f32, u32};
use cgmath::{Matrix4, SquareMatrix, Matrix, Vector3, Vector4, InnerSpace};
use support::Camera;

use embree::Geometry;

/// Make a triangulated sphere, from the Embree tutorial:
/// https://github.com/embree/embree/blob/master/tutorials/instanced_geometry/instanced_geometry_device.cpp
fn make_triangulated_sphere<'a>(scene: &'a embree::Scene, pos: &Vector3<f32>, radius: f32) -> embree::TriangleMesh<'a> {
    let num_phi = 5;
    let num_theta = 2 * num_phi;
    let mut mesh = embree::TriangleMesh::unanimated(scene, embree::GeometryFlags::STATIC,
                                                    2 * num_theta * (num_phi - 1), num_theta * (num_phi + 1));
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
                    tris[tri].x = p10 as i32;
                    tris[tri].y = p00 as i32;
                    tris[tri].z = p01 as i32;
                    tri += 1;
                }
                if phi < num_phi {
                    tris[tri].x = p11 as i32;
                    tris[tri].y = p10 as i32;
                    tris[tri].z = p01 as i32;
                    tri += 1;
                }
            }
        }
    }
    mesh
}
fn make_ground_plane<'a>(scene: &'a embree::Scene) -> embree::QuadMesh<'a> {
    let mut mesh = embree::QuadMesh::unanimated(scene, embree::GeometryFlags::STATIC,
                                                1, 4);
    {
        let mut verts = mesh.vertex_buffer.map();
        let mut quads = mesh.index_buffer.map();
        verts[0] = Vector4::new(-10.0, -2.0, -10.0, 0.0);
        verts[1] = Vector4::new(-10.0, -2.0, 10.0, 0.0);
        verts[2] = Vector4::new(10.0, -2.0, 10.0, 0.0);
        verts[3] = Vector4::new(10.0, -2.0, -10.0, 0.0);

        quads[0] = Vector4::<i32>::new(0, 1, 2, 3);
    }
    mesh
}
// Animate like the Embree example, returns the (transforms, normal_transforms)
fn animate_instances(time: f32, num_instances: usize) -> (Vec<Matrix4<f32>>, Vec<Matrix4<f32>>) {
    let t0 = 0.7 * time;
    let t1 = 1.5 * time;

    let rot = Matrix4::from_cols(Vector4::new(f32::cos(t1), 0.0, f32::sin(t1), 0.0),
                                 Vector4::new(0.0, 1.0, 0.0, 0.0),
                                 Vector4::new(-f32::sin(t1), 0.0, f32::cos(t1), 0.0),
                                 Vector4::new(0.0, 0.0, 0.0, 1.0));

    let mut transforms = Vec::with_capacity(num_instances);
    let mut normal_transforms = Vec::with_capacity(num_instances);
    for i in 0..num_instances {
        let t = t0 + i as f32 * 2.0 * f32::consts::PI / 4.0;
        let trans = Matrix4::<f32>::from_translation(2.2 * Vector3::<f32>::new(f32::cos(t),
                                                                               0.0, f32::sin(t)));
        transforms.push(trans * rot);
        normal_transforms.push(transforms[i].invert().unwrap().transpose());
    }
    (transforms, normal_transforms)
}

fn main() {
    let mut display = support::Display::new(512, 512, "instancing");
    let device = embree::Device::new();
    let scene = embree::Scene::new(&device, embree::SceneFlags::SCENE_DYNAMIC,
                                   embree::AlgorithmFlags::INTERSECT1);

    // Make the scene we'll instance with 4 triangulated spheres
    let instanced_scene = embree::Scene::new(&device, embree::SceneFlags::SCENE_STATIC,
                                             embree::AlgorithmFlags::INTERSECT1);
    let spheres = vec![make_triangulated_sphere(&instanced_scene, &Vector3::new(0.0, 0.0, 1.0), 0.5),
                       make_triangulated_sphere(&instanced_scene, &Vector3::new(1.0, 0.0, 0.0), 0.5),
                       make_triangulated_sphere(&instanced_scene, &Vector3::new(0.0, 0.0, -1.0), 0.5),
                       make_triangulated_sphere(&instanced_scene, &Vector3::new(-1.0, 0.0, 0.0), 0.5)];
    instanced_scene.commit();

    // Make the instances first so their ids will be 0-3 that we can then use
    // directly to index into the instance_colors
    let mut instances = vec![embree::Instance::unanimated(&scene, &instanced_scene),
                         embree::Instance::unanimated(&scene, &instanced_scene),
                         embree::Instance::unanimated(&scene, &instanced_scene),
                         embree::Instance::unanimated(&scene, &instanced_scene)];

    let instance_colors = vec![
        vec![Vector3::new(0.25, 0.0, 0.0), Vector3::new(0.5, 0.0, 0.0),
             Vector3::new(0.75, 0.0, 0.0), Vector3::new(1.00, 0.0, 0.0)],
        vec![Vector3::new(0.0, 0.25, 0.0), Vector3::new(0.0, 0.50, 0.0),
             Vector3::new(0.0, 0.75, 0.0), Vector3::new(0.0, 1.00, 0.0)],
        vec![Vector3::new(0.0, 0.0, 0.25), Vector3::new(0.0, 0.0, 0.50),
             Vector3::new(0.0, 0.0, 0.75), Vector3::new(0.0, 0.0, 1.00)],
        vec![Vector3::new(0.25, 0.25, 0.0), Vector3::new(0.50, 0.50, 0.0),
             Vector3::new(0.75, 0.75, 0.0), Vector3::new(1.00, 1.00, 0.0)]];

    let ground = make_ground_plane(&scene);
    scene.commit();

    let light_dir = Vector3::new(1.0, 1.0, -1.0).normalize();
    display.run(|image, camera_pose, time| {
        for p in image.iter_mut() {
            *p = 0;
        }
        let (transforms, normal_transforms) = animate_instances(time, instances.len());
        for (i, t) in transforms.iter().enumerate() {
            instances[i].set_transform(&t);
            instances[i].update();
        }
        scene.commit();

        let img_dims = image.dimensions();
        let camera = Camera::look_dir(camera_pose.pos, camera_pose.dir,
                                     camera_pose.up, 75.0, img_dims);
        // Render the scene
        for j in 0..img_dims.1 {
            for i in 0..img_dims.0 {
                let dir = camera.ray_dir((i as f32 + 0.5, j as f32 + 0.5));
                let mut ray = embree::Ray::new(&camera.pos, &dir);
                scene.intersect(&mut ray);

                if ray.geomID != u32::MAX {
                    // Transform the normals of the instances into world space with the normal_transforms
                    let normal =
                        if ray.instID == u32::MAX {
                            Vector3::new(ray.Ng[0], ray.Ng[1], ray.Ng[2]).normalize()
                        } else {
                            let v = normal_transforms[ray.instID as usize]
                                    * Vector4::new(ray.Ng[0], ray.Ng[1], ray.Ng[2], 0.0);
                            Vector3::new(v.x, v.y, v.z).normalize()
                        };
                    let mut illum = 0.3;
                    let shadow_pos = camera.pos + dir * ray.tfar;
                    let mut shadow_ray = embree::Ray::new(&shadow_pos, &light_dir);
                    shadow_ray.tnear = 0.001;
                    scene.occluded(&mut shadow_ray);

                    if shadow_ray.geomID != 0 {
                        illum = support::clamp(illum + f32::max(light_dir.dot(normal), 0.0), 0.0, 1.0);
                    }

                    let mut p = image.get_pixel_mut(i, j);
                    // TODO: Why is the operator overload not picked up for this?
                    if ray.instID == u32::MAX && ray.geomID == ground.geom_id() {
                        p.data[0] = (255.0 * illum) as u8;
                        p.data[1] = p.data[0];
                        p.data[2] = p.data[0];
                    } else {
                        // Shade the instances using their color
                        let color = &instance_colors[ray.instID as usize][ray.geomID as usize];
                        p.data[0] = (255.0 * illum * color.x) as u8;
                        p.data[1] = (255.0 * illum * color.y) as u8;
                        p.data[2] = (255.0 * illum * color.z) as u8;
                    }
                }
            }
        }
    });
}

