#![allow(dead_code)]

extern crate embree;
extern crate support;

use embree::{
    BufferSlice, BufferUsage, Device, Format, IntersectContext, QuadMesh, Ray, Scene, TriangleMesh,
    INVALID_ID,
};
use glam::Vec3;
use support::{Camera, Rgba, RgbaImage, TILE_SIZE, TILE_SIZE_X, TILE_SIZE_Y};

fn make_cube(device: &Device, vertex_colors: &[[f32; 3]]) -> TriangleMesh<'static> {
    let mut mesh = TriangleMesh::new(device).unwrap();
    {
        mesh.set_new_buffer(BufferUsage::VERTEX, 0, Format::FLOAT3, 12, 8)
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
        mesh.set_new_buffer(BufferUsage::INDEX, 0, Format::UINT3, 12, 12)
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

        mesh.set_vertex_attribute_count(1);
        mesh.set_buffer(
            BufferUsage::VERTEX_ATTRIBUTE,
            0,
            Format::FLOAT3,
            BufferSlice::from_slice(vertex_colors, ..8),
            12,
            8,
        )
        .unwrap(); //.expect("failed to set vertex attribute buffer");
    }
    mesh.commit();
    mesh
}

fn make_ground_plane(device: &Device) -> QuadMesh<'static> {
    let mut mesh = QuadMesh::new(device).unwrap();
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

struct State {
    ground_id: u32,
    cube_id: u32,
    face_colors: Vec<[f32; 3]>,
    light_dir: Vec3,
    scene: Scene<'static>,
}

fn main() {
    let display = support::Display::new(512, 512, "triangle geometry");
    let device = Device::new().unwrap();
    device.set_error_function(|err, msg| {
        println!("{}: {}", err, msg);
    });
    let vertex_colors = vec![
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 1.0],
        [1.0, 1.0, 0.0],
        [1.0, 1.0, 1.0],
    ];

    let mut state = State {
        face_colors: vec![
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.5, 0.5, 0.5],
            [0.5, 0.5, 0.5],
            [1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ],
        ground_id: INVALID_ID,
        cube_id: INVALID_ID,
        light_dir: Vec3::new(1.0, 1.0, 1.0).normalize(),
        scene: device.create_scene().unwrap(),
    };

    let cube = make_cube(&device, &vertex_colors);
    let ground = make_ground_plane(&device);
    state.cube_id = state.scene.attach_geometry(&cube);
    state.ground_id = state.scene.attach_geometry(&ground);
    state.scene.commit();

    let mut last_time = 0.0;
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

        //render_frame(image, time, &camera, &state);

        // Render the scene
        for j in 0..img_dims.1 {
            for i in 0..img_dims.0 {
                render_pixel(i, j, &mut image.get_pixel_mut(i, j), time, &camera, &state);
            }
        }

        let elapsed = time - last_time;
        last_time = time;
        let fps = 1.0 / elapsed;
        eprint!("\r{} fps", fps);
    });
}

// Task that renders a single pixel.
fn render_pixel(x: u32, y: u32, pixel: &mut Rgba<u8>, _time: f32, camera: &Camera, state: &State) {
    let mut ctx = IntersectContext::coherent();
    let dir = camera.ray_dir((x as f32 + 0.5, y as f32 + 0.5));
    let ray_hit = state
        .scene
        .intersect(&mut ctx, Ray::new(camera.pos.into(), dir.into()));
    if ray_hit.is_valid() {
        let diffuse = if ray_hit.hit.geomID == state.ground_id {
            glam::vec3(0.6, 0.6, 0.6)
        } else {
            glam::Vec3::from(state.face_colors[ray_hit.hit.primID as usize])
        };

        let mut shadow_ray = Ray::segment(
            ray_hit.hit_point(),
            state.light_dir.into(),
            0.001,
            f32::INFINITY,
        );

        // Check if the shadow ray is occluded.
        let color = if !state.scene.occluded(&mut ctx, &mut shadow_ray) {
            diffuse
        } else {
            diffuse * 0.5
        };

        // Write the color to the image.
        pixel[0] = (color.x * 255.0) as u8;
        pixel[1] = (color.y * 255.0) as u8;
        pixel[2] = (color.z * 255.0) as u8;
    }
}

fn render_tile(
    tile_idx: u32,
    num_tiles_x: u32,
    num_tiles_y: u32,
    width: u32,
    height: u32,
    time: f32,
    camera: &Camera,
    state: &State,
    image: &mut RgbaImage,
) {
    let title_y = tile_idx / num_tiles_x;
    let tile_x = tile_idx % num_tiles_x;
    let x0 = tile_x * TILE_SIZE_X;
    let y0 = title_y * TILE_SIZE_Y;
    let x1 = (x0 + TILE_SIZE_X).min(width);
    let y1 = (y0 + TILE_SIZE_Y).min(height);

    for y in y0..y1 {
        for x in x0..x1 {
            render_pixel(x, y, &mut image.get_pixel_mut(x, y), time, &camera, &state);
        }
    }
}

fn render_frame(image: &mut RgbaImage, time: f32, camera: &Camera, state: &State) {
    use rayon::prelude::*;
    let img_dims = image.dimensions();
    let num_tiles_x = (img_dims.0 + TILE_SIZE_X - 1) / TILE_SIZE_X;
    let num_tiles_y = (img_dims.1 + TILE_SIZE_Y - 1) / TILE_SIZE_Y;
    let num_tiles = num_tiles_x * num_tiles_y;

    (0..num_tiles).into_par_iter().for_each(|tile_idx| {
        render_tile(
            tile_idx,
            num_tiles_x,
            num_tiles_y,
            img_dims.0,
            img_dims.1,
            time,
            camera,
            state,
            image,
        );
    });
}
