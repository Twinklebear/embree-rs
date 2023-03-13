#![allow(dead_code)]

extern crate embree;
extern crate support;

use embree::{
    BufferSlice, BufferUsage, Device, Format, IntersectContext, QuadMesh, Ray, RayHit,
    TriangleMesh, INVALID_ID,
};
use glam::Vec3;
use support::*;

const DISPLAY_WIDTH: u32 = 512;
const DISPLAY_HEIGHT: u32 = 512;

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
                [0, 1, 2],
                [1, 3, 2],
                // right side
                [4, 6, 5],
                [5, 6, 7],
                // bottom side
                [0, 4, 1],
                [1, 4, 5],
                // top side
                [2, 3, 6],
                [3, 7, 6],
                // front side
                [0, 2, 4],
                [2, 6, 4],
                // back side
                [1, 5, 3],
                [3, 5, 7],
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

type State = DebugState<UserState>;

struct UserState {
    ground_id: u32,
    cube_id: u32,
    face_colors: Vec<[f32; 3]>,
    light_dir: Vec3,
}

fn main() {
    let display = Display::new(DISPLAY_WIDTH, DISPLAY_HEIGHT, "triangle geometry");
    let device = Device::new().unwrap();
    device.set_error_function(|err, msg| {
        println!("{}: {}", err, msg);
    });
    let scene = device.create_scene().unwrap();
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

    let user_state = UserState {
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
    };

    let mut state = State {
        scene: scene.clone(),
        user: user_state,
    };

    let cube = make_cube(&device, &vertex_colors);
    let ground = make_ground_plane(&device);
    state.user.cube_id = state.scene.attach_geometry(&cube);
    state.user.ground_id = state.scene.attach_geometry(&ground);

    state.scene.commit();

    display::run(display, state, move |_, _| {}, render_frame, |_| {});
}

// Task that renders a single pixel.
fn render_pixel(x: u32, y: u32, _time: f32, camera: &Camera, state: &State) -> u32 {
    let mut ctx = IntersectContext::coherent();
    let dir = camera.ray_dir((x as f32 + 0.5, y as f32 + 0.5));
    let mut ray_hit = RayHit::from_ray(Ray::segment(
        camera.pos.into(),
        dir.into(),
        0.0,
        f32::INFINITY,
    ));
    state.scene.intersect(&mut ctx, &mut ray_hit);
    let mut pixel = 0;
    if ray_hit.hit.is_valid() {
        let diffuse = if ray_hit.hit.geomID == state.user.ground_id {
            glam::vec3(0.6, 0.6, 0.6)
        } else {
            glam::Vec3::from(state.user.face_colors[ray_hit.hit.primID as usize])
        };

        let mut shadow_ray = Ray::segment(
            ray_hit.ray.hit_point(),
            state.user.light_dir.into(),
            0.001,
            f32::INFINITY,
        );

        // Check if the shadow ray is occluded.
        let color = if !state.scene.occluded(&mut ctx, &mut shadow_ray) {
            diffuse
        } else {
            diffuse * 0.5
        };

        pixel = rgba_to_u32(
            (color.x * 255.0) as u8,
            (color.y * 255.0) as u8,
            (color.z * 255.0) as u8,
            255,
        );
    }
    pixel
}

fn render_frame(frame: &mut TiledImage, camera: &Camera, time: f32, state: &mut State) {
    frame.par_tiles_mut().for_each(|tile| {
        tile.pixels.iter_mut().enumerate().for_each(|(i, pixel)| {
            let x = tile.x + (i % tile.w as usize) as u32;
            let y = tile.y + (i / tile.w as usize) as u32;
            *pixel = render_pixel(x, y, time, camera, state);
        });
    });
}
