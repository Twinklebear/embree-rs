//! This example show how to create a dynamic scene.

use embree::{
    BufferUsage, BuildQuality, Device, Format, Geometry, IntersectContext, Ray, Scene, SceneFlags,
};
use support::Camera;
use glam::{Vec3, vec3};

const NUM_SPHERES: usize = 20;
const NUM_PHI: usize = 120;
const NUM_THETA: usize = 2 * NUM_PHI;

fn create_sphere(
    device: &Device,
    quality: BuildQuality,
    pos: Vec3,
    radius: f32,
) -> Geometry<'static> {
    // Create a triangulated sphere
    let mut geometry = device
        .create_geometry(embree::GeometryType::TRIANGLE)
        .unwrap();
    geometry.set_build_quality(quality);

    let mut vertices = geometry
        .set_new_buffer(
            BufferUsage::VERTEX,
            0,
            Format::FLOAT3,
            16,
            NUM_THETA * (NUM_PHI + 1),
        )
        .unwrap()
        .view_mut::<[f32; 4]>()
        .unwrap();

    let mut indices = geometry
        .set_new_buffer(
            BufferUsage::INDEX,
            0,
            Format::UINT3,
            12,
            2 * NUM_THETA * (NUM_PHI - 1),
        )
        .unwrap()
        .view_mut::<[u32; 3]>()
        .unwrap();

    let mut tri = 0;
    let rcp_num_theta = 1.0 / NUM_THETA as f32;
    let rcp_num_phi = 1.0 / NUM_PHI as f32;
    for phi_idx in 0..NUM_PHI {
        for theta_idx in 0..NUM_THETA {
            let phi = phi_idx as f32 * rcp_num_phi * std::f32::consts::PI;
            let theta = theta_idx as f32 * rcp_num_theta * 2.0 * std::f32::consts::PI;
            vertices[phi_idx * NUM_THETA + theta_idx] = [
                pos.x + radius * phi.sin() * theta.sin(),
                pos.y + radius * phi.cos(),
                pos.z + radius * phi.sin() * theta.cos(),
                0.0,
            ];
        }
        if phi_idx == 0 {
            continue;
        }

        for theta_idx in 1..=NUM_THETA {
            let p00 = ((phi_idx - 1) * NUM_THETA + theta_idx - 1) as u32;
            let p01 = ((phi_idx - 1) * NUM_THETA + theta_idx % NUM_THETA) as u32;
            let p10 = (phi_idx * NUM_THETA + theta_idx - 1) as u32;
            let p11 = (phi_idx * NUM_THETA + theta_idx % NUM_THETA) as u32;

            if phi_idx > 1 {
                indices[tri] = [p10, p01, p00];
                tri += 1;
            }

            if phi_idx < NUM_PHI {
                indices[tri] = [p11, p01, p10];
                tri += 1;
            }
        }
    }
    geometry.commit();
    geometry
}

fn create_ground_plane(device: &Device) -> Geometry<'static> {
    let mut geometry = Geometry::new(device, embree::GeometryType::TRIANGLE).unwrap();
    {
        geometry
            .set_new_buffer(BufferUsage::VERTEX, 0, Format::FLOAT3, 16, 4)
            .unwrap()
            .view_mut::<[f32; 4]>()
            .unwrap()
            .copy_from_slice(&[
                [-10.0, -2.0, -10.0, 0.0],
                [-10.0, -2.0, 10.0, 0.0],
                [10.0, -2.0, -10.0, 0.0],
                [10.0, -2.0, 10.0, 0.0],
            ]);
        geometry
            .set_new_buffer(BufferUsage::INDEX, 0, Format::UINT3, 12, 2)
            .unwrap()
            .view_mut::<[u32; 3]>()
            .unwrap()
            .copy_from_slice(&[[0, 1, 2], [1, 3, 2]]);
    }
    geometry.commit();
    geometry
}

fn animate_sphere(scene: &Scene, id: u32, pos: Vec3, radius: f32, time: f32) {
    let mut geometry = scene.get_geometry(id).unwrap();
    let mut vertices = geometry
        .get_buffer(BufferUsage::VERTEX, 0)
        .unwrap()
        .view_mut::<[f32; 4]>()
        .unwrap();
    let num_theta_rcp = 1.0 / NUM_THETA as f32;
    let num_phi_rcp = 1.0 / NUM_PHI as f32;
    let f = 2.0 * (1.0 + 0.5 * time.sin());

    #[cfg(feature = "rayon")]
    {
        use rayon::prelude::*;
        vertices
            .par_chunks_mut(NUM_THETA)
            .enumerate()
            .for_each(|(phi_idx, chunk)| {
                let phi = phi_idx as f32 * num_phi_rcp * std::f32::consts::PI;
                for (theta_idx, v) in chunk.iter_mut().enumerate() {
                    let theta = theta_idx as f32 * num_theta_rcp * 2.0 * std::f32::consts::PI;
                    v[0] = pos.x + radius * (f * phi).sin() * theta.sin();
                    v[1] = pos.y + radius * phi.cos();
                    v[2] = pos.z + radius * (f * phi).sin() * theta.cos();
                }
            });
    }
    #[cfg(not(feature = "rayon"))]
    {
        for phi_idx in 0..NUM_PHI {
            for theta_idx in 0..NUM_THETA {
                let phi = phi_idx as f32 * num_phi_rcp * std::f32::consts::PI;
                let theta = theta_idx as f32 * num_theta_rcp * 2.0 * std::f32::consts::PI;
                let mut v = vertices[phi_idx * NUM_THETA + theta_idx];
                v[0] = pos.x + radius * (f * phi).sin() * theta.sin();
                v[1] = pos.y + radius * phi.cos();
                v[2] = pos.z + radius * (f * phi).sin() * theta.cos();
            }
        }
    }
    geometry.update_buffer(BufferUsage::VERTEX, 0);
    geometry.commit();
}

fn main() {
    let device = Device::new().unwrap();
    device.set_error_function(|err, msg| {
        eprintln!("{}: {}", err, msg);
    });
    let mut scene = device.create_scene().unwrap();
    scene.set_flags(SceneFlags::DYNAMIC | SceneFlags::ROBUST);
    scene.set_build_quality(BuildQuality::LOW);

    let mut positions = [Vec3::ZERO; NUM_SPHERES];
    let mut radii = [1.0; NUM_SPHERES];
    let mut colors = [Vec3::ZERO; NUM_SPHERES + 1];

    // Create a few triangulated spheres.
    for i in 0..NUM_SPHERES {
        let phi = i as f32 / NUM_SPHERES as f32 * std::f32::consts::PI * 2.0;
        let radius = 2.0 * std::f32::consts::PI / NUM_SPHERES as f32;
        let pos = 2.0 * Vec3::new(phi.sin(), 0.0, -phi.cos());
        let quality = if i % 2 == 0 {
            BuildQuality::LOW
        } else {
            BuildQuality::REFIT
        };
        let sphere = create_sphere(&device, quality, pos, radius);
        let id = scene.attach_geometry(&sphere);
        positions[id as usize] = pos;
        radii[id as usize] = radius;
        colors[id as usize] = Vec3::new(
            (i % 16 + 1) as f32 / 17.0,
            (i % 8 + 1) as f32 / 9.0,
            (i % 4 + 1) as f32 / 5.0,
        );
    }
    let id = scene.attach_geometry(&create_ground_plane(&device));
    colors[id as usize] = Vec3::new(1.0, 1.0, 1.0);
    scene.commit();

    let display = support::Display::new(512, 512, "Dynamic Scene");
    let light_dir = vec3(1.0, 1.0, 1.0).normalize();
    let mut time = 0;
    support::display::run(display, move |image, camera_pose, _| {
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

        for i in 0..NUM_SPHERES {
            animate_sphere(&scene, i as u32, positions[i], radii[i], time as f32);
        }
        scene.commit();

        // Render the scene
        for j in 0..img_dims.1 {
            for i in 0..img_dims.0 {
                let dir = camera.ray_dir((i as f32 + 0.5, j as f32 + 0.5));
                let mut intersection_ctx = IntersectContext::coherent();
                let ray_hit = scene.intersect(
                    &mut intersection_ctx,
                    Ray::new(camera.pos.into(), dir.into()),
                );

                if ray_hit.is_valid() {
                    let p = image.get_pixel_mut(i, j);
                    let diffuse = colors[ray_hit.hit.geomID as usize];

                    let mut shadow_ray =
                        Ray::segment(ray_hit.hit_point(), light_dir.into(), 0.001, f32::INFINITY);

                    // Check if the shadow ray is occluded.
                    let color = if !scene.occluded(&mut intersection_ctx, &mut shadow_ray) {
                        diffuse
                    } else {
                        diffuse * 0.5
                    };

                    // Write the color to the image.
                    p[0] = (color.x * 255.0) as u8;
                    p[1] = (color.y * 255.0) as u8;
                    p[2] = (color.z * 255.0) as u8;
                }
            }
        }
        time += 1;
    });
}
