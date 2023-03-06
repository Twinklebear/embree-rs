#![allow(dead_code)]
extern crate embree;
extern crate support;

use cgmath::{InnerSpace, Matrix, Matrix4, SquareMatrix, Vector3, Vector4};
use embree::{
    BufferUsage, BuildQuality, Device, Format, Geometry, Instance, IntersectContext, Ray, RayHit,
    Scene, SceneFlags, INVALID_ID,
};
use support::{
    rgba_to_u32, Camera, ParallelIterator, RgbaImage, TiledImage, DEFAULT_DISPLAY_WIDTH,
    TILE_SIZE_X, TILE_SIZE_Y,
};

const NUM_PHI: usize = 5;
const NUM_THETA: usize = 2 * NUM_PHI;

const COLORS: [[[f32; 3]; 4]; 4] = [
    [
        [0.25, 0.0, 0.0],
        [0.5, 0.0, 0.0],
        [0.75, 0.0, 0.0],
        [1.0, 0.0, 0.0],
    ],
    [
        [0.0, 0.25, 0.0],
        [0.0, 0.5, 0.0],
        [0.0, 0.75, 0.0],
        [0.0, 1.0, 0.0],
    ],
    [
        [0.0, 0.0, 0.25],
        [0.0, 0.0, 0.5],
        [0.0, 0.0, 0.75],
        [0.0, 0.0, 1.0],
    ],
    [
        [0.25, 0.25, 0.0],
        [0.5, 0.5, 0.0],
        [0.75, 0.75, 0.0],
        [1.0, 1.0, 0.0],
    ],
];

fn create_sphere(device: &Device, pos: Vector3<f32>, radius: f32) -> Geometry<'static> {
    // Create a triangulated sphere
    let mut geometry = device
        .create_geometry(embree::GeometryKind::TRIANGLE)
        .unwrap();
    geometry.set_build_quality(BuildQuality::LOW);

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
    let mut geometry = Geometry::new(device, embree::GeometryKind::TRIANGLE).unwrap();
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

// Animate like the Embree example.
fn animate_instances(
    time: f32,
    num_instances: usize,
    transforms: &mut [Matrix4<f32>],
    normal_transforms: &mut [Matrix4<f32>],
) {
    debug_assert!(transforms.len() == num_instances);
    debug_assert!(normal_transforms.len() == num_instances);

    let t0 = 0.7 * time;
    let t1 = 1.5 * time;

    let rot = Matrix4::from_cols(
        Vector4::new(f32::cos(t1), 0.0, f32::sin(t1), 0.0),
        Vector4::new(0.0, 1.0, 0.0, 0.0),
        Vector4::new(-f32::sin(t1), 0.0, f32::cos(t1), 0.0),
        Vector4::new(0.0, 0.0, 0.0, 1.0),
    );
    for i in 0..num_instances {
        let t = t0 + i as f32 * 2.0 * std::f32::consts::PI / 4.0;
        let trans = Matrix4::<f32>::from_translation(
            2.2 * Vector3::<f32>::new(f32::cos(t), 0.0, f32::sin(t)),
        );
        transforms[i] = trans * rot;
        normal_transforms[i] = transforms[i].invert().unwrap().transpose();
    }
}

struct State {
    transforms: Vec<Matrix4<f32>>,
    normal_transforms: Vec<Matrix4<f32>>,
    ground_plane_id: u32,
    light_dir: Vector3<f32>,
}

fn main() {
    let display = support::Display::new(512, 512, "instancing");
    let device = Device::new().unwrap();

    // Create a scene.
    let mut scene = device.create_scene().unwrap();
    scene.set_build_quality(BuildQuality::LOW);
    scene.set_flags(SceneFlags::DYNAMIC);

    // Create a scene with 4 triangulated spheres.
    let mut scene1 = device.create_scene().unwrap();
    let spheres = vec![
        create_sphere(&device, Vector3::new(0.0, 0.0, 1.0), 0.5),
        create_sphere(&device, Vector3::new(1.0, 0.0, 0.0), 0.5),
        create_sphere(&device, Vector3::new(0.0, 0.0, -1.0), 0.5),
        create_sphere(&device, Vector3::new(-1.0, 0.0, 0.0), 0.5),
    ];
    for s in spheres.into_iter() {
        scene1.attach_geometry(&s);
    }
    scene1.commit();

    // Instantiate geometries
    let mut instances = vec![
        Instance::new(&device).unwrap(),
        Instance::new(&device).unwrap(),
        Instance::new(&device).unwrap(),
        Instance::new(&device).unwrap(),
    ];

    for inst in instances.iter_mut() {
        inst.set_instanced_scene(&scene1);
        inst.set_time_step_count(1);
        inst.commit();
        scene.attach_geometry(&inst);
    }
    scene.commit();

    let ground_plane = create_ground_plane(&device);
    let ground_plane_id = scene.attach_geometry(&ground_plane);

    let mut state = State {
        transforms: vec![Matrix4::identity(); instances.len()],
        normal_transforms: vec![Matrix4::identity(); instances.len()],
        ground_plane_id,
        light_dir: Vector3::new(1.0, 1.0, -1.0).normalize(),
    };

    let mut tiled = TiledImage::new(
        DEFAULT_DISPLAY_WIDTH,
        DEFAULT_DISPLAY_WIDTH,
        TILE_SIZE_X,
        TILE_SIZE_Y,
    );

    let mut last_time = 0.0;

    support::display::run(display, move |image, camera_pose, time| {
        for p in image.iter_mut() {
            *p = 0;
        }
        // Update scene transformations
        animate_instances(
            time,
            instances.len(),
            &mut state.transforms,
            &mut state.normal_transforms,
        );
        for (inst, tfm) in instances.iter_mut().zip(state.transforms.iter()) {
            inst.set_transform(0, tfm.as_ref());
            inst.commit();
        }
        scene.commit();

        let img_dims = image.dimensions();
        let camera = Camera::look_dir(
            camera_pose.pos,
            camera_pose.dir,
            camera_pose.up,
            55.0,
            img_dims,
        );

        render_frame(&mut tiled, image, time, &scene, &camera, &state);

        let elapsed = time - last_time;
        last_time = time;
        let fps = 1.0 / elapsed;
        eprint!("\r{} fps", fps);
    });
}

fn render_pixel(
    x: u32,
    y: u32,
    pixel: &mut u32,
    _time: f32,
    scene: &Scene,
    camera: &Camera,
    state: &State,
) {
    let mut ctx = IntersectContext::coherent();
    let dir = camera.ray_dir((x as f32 + 0.5, y as f32 + 0.5));
    let mut ray_hit = RayHit::from_ray(Ray::segment(
        camera.pos.into(),
        dir.into(),
        0.001,
        f32::INFINITY,
    ));
    scene.intersect(&mut ctx, &mut ray_hit);

    if ray_hit.hit.is_valid() {
        // Transform the normals of the instances into world space with the
        // normal_transforms
        let hit = &ray_hit.hit;
        let geom_id = hit.geomID;
        let inst_id = hit.instID[0];
        let mut normal = Vector3::from(hit.unit_normal());
        if inst_id != INVALID_ID {
            let v = state.normal_transforms[inst_id as usize]
                * Vector4::new(normal.x, normal.y, normal.z, 0.0);
            normal = Vector3::new(v.x, v.y, v.z).normalize()
        }
        let mut illum = 0.3;
        let shadow_pos = camera.pos + dir * ray_hit.ray.tfar;
        let mut shadow_ray = Ray::segment(
            shadow_pos.into(),
            state.light_dir.into(),
            0.001,
            f32::INFINITY,
        );
        scene.occluded(&mut ctx, &mut shadow_ray);

        if shadow_ray.tfar >= 0.0 {
            illum = support::clamp(illum + f32::max(state.light_dir.dot(normal), 0.0), 0.0, 1.0);
        }

        *pixel = if inst_id == INVALID_ID && geom_id == state.ground_plane_id {
            rgba_to_u32(
                (255.0 * illum) as u8,
                (255.0 * illum) as u8,
                (255.0 * illum) as u8,
                255,
            )
        } else {
            // Shade the instances using their color
            let color = &COLORS[inst_id as usize][geom_id as usize];
            rgba_to_u32(
                (255.0 * illum * color[0]) as u8,
                (255.0 * illum * color[1]) as u8,
                (255.0 * illum * color[2]) as u8,
                255,
            )
        }
    }
}

fn render_frame(
    tiled: &mut TiledImage,
    frame: &mut RgbaImage,
    time: f32,
    scene: &Scene,
    camera: &Camera,
    state: &State,
) {
    tiled.reset_pixels();
    tiled.par_tiles_mut().for_each(|tile| {
        tile.pixels.iter_mut().enumerate().for_each(|(i, pixel)| {
            let x = tile.x + (i % tile.w as usize) as u32;
            let y = tile.y + (i / tile.w as usize) as u32;
            render_pixel(x, y, pixel, time, scene, camera, &state);
        });
    });
    tiled.write_to_image(frame);
}
