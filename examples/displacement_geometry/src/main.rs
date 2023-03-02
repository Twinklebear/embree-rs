use embree::{
    BufferSlice, BufferUsage, Device, Format, Geometry, GeometryKind, InterpolateInput,
    InterpolateOutput, IntersectContext, Ray, RayHit, Scene, SceneFlags,
};
use glam::{vec3, Vec3};
use support::{
    noise, rgba_to_u32, Align16Array, Camera, ParallelIterator, RgbaImage, TiledImage,
    DEFAULT_DISPLAY_HEIGHT, DEFAULT_DISPLAY_WIDTH, TILE_SIZE_X, TILE_SIZE_Y,
};

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

const LIGHT_DIR: [f32; 3] = [0.57; 3];

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

fn displacement_du_or_dv(p: Vec3, dp_du_or_dp_dv: Vec3) -> f32 {
    let du_or_dv = 0.001;
    (displacement((p + du_or_dv * dp_du_or_dp_dv).into()) - displacement(p.into())) / du_or_dv
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
                for (_, ng, p) in vertices.into_iter_mut() {
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

    let display = support::Display::new(
        DEFAULT_DISPLAY_WIDTH,
        DEFAULT_DISPLAY_HEIGHT,
        "Dynamic Scene",
    );
    let mut last_time = 0.0;
    let mut tiled = TiledImage::new(
        DEFAULT_DISPLAY_WIDTH,
        DEFAULT_DISPLAY_HEIGHT,
        TILE_SIZE_X,
        TILE_SIZE_Y,
    );
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

        render_frame(&mut tiled, image, &camera, &scene, cube_id, ground_id);

        let elapsed = time - last_time;
        last_time = time;
        let fps = 1.0 / elapsed;
        eprint!("\r{} fps", fps);
    });
}

fn render_pixel(
    x: u32,
    y: u32,
    camera: &Camera,
    scene: &Scene,
    cube_id: u32,
    ground_id: u32,
) -> u32 {
    let dir = camera.ray_dir((x as f32 + 0.5, y as f32 + 0.5));
    let mut ctx = IntersectContext::coherent();
    let mut ray_hit = RayHit::from_ray(Ray::new(camera.pos.into(), dir.into()));
    scene.intersect(&mut ctx, &mut ray_hit);

    let mut color = vec3(0.0, 0.0, 0.0);
    if ray_hit.is_valid() {
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
                let cube = scene.get_geometry_unchecked(cube_id).unwrap();
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
                let mut dp_du = glam::Vec3::from_slice(output.dp_du().as_ref().unwrap());
                let mut dp_dv = glam::Vec3::from_slice(output.dp_dv().as_ref().unwrap());
                let ng = dp_du.cross(dp_dv).normalize();
                dp_du += ng * displacement_du_or_dv(hit_point, dp_du);
                dp_dv += ng * displacement_du_or_dv(hit_point, dp_dv);
                normal = dp_du.cross(dp_dv).normalize();
            }
        }

        let mut shadow_ray = Ray::segment(ray_hit.hit_point(), LIGHT_DIR, 0.001, f32::INFINITY);

        // Check if the shadow ray is occluded.
        if !scene.occluded(&mut ctx, &mut shadow_ray) {
            color += diffuse * glam::Vec3::from(LIGHT_DIR).dot(normal).clamp(0.0, 1.0);
        }
    }

    rgba_to_u32(
        (color.x * 255.0) as u8,
        (color.y * 255.0) as u8,
        (color.z * 255.0) as u8,
        255,
    )
}

fn render_frame(
    tiled: &mut TiledImage,
    frame: &mut RgbaImage,
    camera: &Camera,
    scene: &Scene,
    cube_id: u32,
    ground_id: u32,
) {
    tiled.reset_pixels();
    tiled.par_tiles_mut().for_each(|tile| {
        tile.pixels.iter_mut().enumerate().for_each(|(i, pixel)| {
            let x = tile.x + (i % tile.w as usize) as u32;
            let y = tile.y + (i / tile.w as usize) as u32;
            *pixel = render_pixel(x, y, camera, scene, cube_id, ground_id);
        });
    });
    tiled.write_to_image(frame);
}
