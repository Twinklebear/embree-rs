use embree::{
    BufferSlice, BufferUsage, Device, Format, Geometry, GeometryKind, InterpolateInput,
    InterpolateOutput, IntersectContext, Ray, Scene, SceneFlags,
};
use glam::vec3;
use support::{noise, Align16Array, Camera};

const EDGE_LEVEL: f32 = 256.0;
const NUM_INDICES: usize = 24;
const NUM_FACES: usize = 6;
const FACE_SIZE: usize = 4;

const CUBE_VERTICES: Align16Array<f32, 32> = Align16Array([
    -1.0, -1.0, -1.0, 0.0, // 0
    1.0, -1.0, -1.0, 0.0, // 1
    1.0, -1.0, 1.0, 0.0, // 2
    -1.0, -1.0, 1.0, 0.0, // 3
    -1.0, 1.0, -1.0, 0.0, // 4
    1.0, 1.0, -1.0, 0.0, // 5
    1.0, 1.0, 1.0, 0.0, // 6
    -1.0, 1.0, 1.0, 0.0, // 7
]);

const CUBE_INDICES: [u32; NUM_INDICES] = [
    0, 4, 5, 1, // 0
    1, 5, 6, 2, // 1
    2, 6, 7, 3, // 2
    0, 3, 7, 4, // 3
    4, 7, 6, 5, // 4
    0, 1, 2, 3, // 5
];

const CUBE_FACES: [u32; NUM_FACES] = [4; 6];

fn displacement(p: [f32; 3]) -> f32 {
    let mut dn = 0.0;
    let mut freq = 1.0;
    while freq < 40.0 {
        let n = noise([p[0] * freq, p[1] * freq, p[2] * freq]).abs();
        dn += 1.4 * n * n / freq;
        freq *= 2.0;
    }
    dn
}

fn create_cube(device: &Device) -> Geometry<'static> {
    let mut geom = device.create_geometry(GeometryKind::SUBDIVISION).unwrap();
    geom.set_buffer(
        BufferUsage::VERTEX,
        0,
        Format::FLOAT3,
        BufferSlice::from_slice(&CUBE_VERTICES.0, ..),
        4 * std::mem::size_of::<f32>(),
        8,
    )
    .unwrap();
    geom.set_buffer(
        BufferUsage::INDEX,
        0,
        Format::UINT,
        BufferSlice::from_slice(&CUBE_INDICES, ..),
        std::mem::size_of::<u32>(),
        NUM_INDICES,
    )
    .unwrap();
    geom.set_buffer(
        BufferUsage::FACE,
        0,
        Format::UINT,
        BufferSlice::from_slice(&CUBE_FACES, ..),
        std::mem::size_of::<u32>(),
        NUM_FACES,
    )
    .unwrap();

    geom.set_new_buffer(
        BufferUsage::LEVEL,
        0,
        Format::FLOAT,
        std::mem::size_of::<f32>(),
        NUM_INDICES,
    )
    .unwrap()
    .view_mut::<f32>()
    .unwrap()
    .copy_from_slice(&[EDGE_LEVEL; NUM_INDICES]);
    unsafe {
        geom.set_displacement_function(
            |raw_geom, user_data: Option<&mut ()>, prim_id, _, vertices| {
                for (uv, ng, p) in vertices.into_iter_mut() {
                    let disp = displacement([*p[0], *p[1], *p[2]]);
                    let dp = [disp * ng[0], disp * ng[1], disp * ng[2]];
                    *p[0] += dp[0];
                    *p[1] += dp[1];
                    *p[2] += dp[2];
                }
            },
        );
    }
    geom.commit();
    geom
}

fn create_ground_plane(device: &Device) -> Geometry<'static> {
    let mut mesh = device.create_geometry(GeometryKind::QUAD).unwrap();
    {
        mesh.set_new_buffer(BufferUsage::VERTEX, 0, Format::FLOAT3, 16, 4)
            .unwrap()
            .view_mut::<[f32; 4]>()
            .unwrap()
            .copy_from_slice(&[
                [-10.0, -2.0, -10.0, 0.0],
                [-10.0, -2.0, 10.0, 0.0],
                [10.0, -2.0, 10.0, 0.0],
                [10.0, -2.0, -10.0, 0.0],
            ]);
        mesh.set_new_buffer(BufferUsage::INDEX, 0, Format::UINT4, 16, 1)
            .unwrap()
            .view_mut::<[u32; 4]>()
            .unwrap()
            .copy_from_slice(&[[0, 1, 2, 3]]);
    }
    mesh.commit();
    mesh
}

fn main() {
    let device = Device::new().unwrap();
    device.set_error_function(|err, msg| {
        eprintln!("{}: {}", err, msg);
    });
    let mut scene = device.create_scene().unwrap();
    scene.set_flags(SceneFlags::ROBUST);

    let cube = create_cube(&device);
    let ground = create_ground_plane(&device);

    let ground_id = scene.attach_geometry(&ground);
    let cube_id = scene.attach_geometry(&cube);
    scene.commit();

    let display = support::Display::new(512, 512, "Dynamic Scene");
    let light_dir = vec3(1.0, 1.0, 1.0).normalize();
    support::display::run(display, move |image, camera_pose, time| {
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
                let mut intersection_ctx = IntersectContext::coherent();
                let ray_hit = scene.intersect(
                    &mut intersection_ctx,
                    Ray::new(camera.pos.into(), dir.into()),
                );

                let mut color = vec3(0.0, 0.0, 0.0);
                if ray_hit.is_valid() {
                    let pixel = image.get_pixel_mut(i, j);
                    let diffuse = if ray_hit.hit.geomID == cube_id {
                        vec3(0.9, 0.6, 0.5)
                    } else {
                        vec3(1.0, 0.0, 0.0)
                    };
                    color += diffuse * 0.5;

                    let mut normal = glam::Vec3::from(ray_hit.hit.normal_normalized());

                    #[cfg(feature = "smooth_normals")]
                    {
                        let hit_point: glam::Vec3 = ray_hit.hit_point().into();
                        if ray_hit.hit.geomID != ground_id {
                            let mut output = InterpolateOutput::new(3, true, true, false);
                            cube.interpolate(
                                InterpolateInput {
                                    prim_id: ray_hit.hit.primID,
                                    u: ray_hit.hit.u,
                                    v: ray_hit.hit.v,
                                    usage: BufferUsage::VERTEX,
                                    slot: 0,
                                },
                                &mut output,
                            );
                            let mut dp_du =
                                glam::Vec3::from_slice(output.dp_du().as_ref().unwrap());
                            let mut dp_dv =
                                glam::Vec3::from_slice(output.dp_dv().as_ref().unwrap());
                            let ng = dp_du.cross(dp_dv).normalize();
                            dp_du += ng * displacement([hit_point.x, hit_point.y, hit_point.z]);
                            dp_dv += ng * displacement([hit_point.x, hit_point.y, hit_point.z]);
                            normal = dp_du.cross(dp_dv).normalize();
                        }
                    }

                    let mut shadow_ray =
                        Ray::segment(ray_hit.hit_point(), light_dir.into(), 0.001, f32::INFINITY);

                    // Check if the shadow ray is occluded.
                    if !scene.occluded(&mut intersection_ctx, &mut shadow_ray) {
                        color += diffuse * light_dir.dot(normal).clamp(0.0, 1.0);
                    }

                    // Write the color to the image.
                    pixel[0] = (color.x * 255.0) as u8;
                    pixel[1] = (color.y * 255.0) as u8;
                    pixel[2] = (color.z * 255.0) as u8;
                }
            }
        }
    });
}
