//! This example shows how to use filter callback functions to efficiently
//! implement transparent objects.
//!
//! The filter function is used for primary rays lets the ray pass through
//! the geometry if it is entirely transparent. Otherwise, the shading loop
//! handles the transparency properly, by potentially shooting secondary rays.
//!
//! The filter function used for shadow rays accumulates the transparency of
//! all surfaces along the ray, and terminates traversal if an opaque surface
//! occluder is hit.

use embree::{
    BufferSlice, BufferUsage, BuildQuality, Device, Format, Geometry, GeometryKind, HitN,
    IntersectContext, Ray, RayHit, RayN, Scene, ValidMasks,
};
use glam::{vec3, Mat3, Vec3};
use support::{
    rgba_to_u32, Align16Array, Camera, Mode, ParallelIterator, RgbaImage, Tile, TiledImage,
    DEFAULT_DISPLAY_HEIGHT, DEFAULT_DISPLAY_WIDTH, TILE_SIZE_X, TILE_SIZE_Y,
};

const CUBE_NUM_VERTICES: usize = 8;
const CUBE_NUM_QUAD_INDICES: usize = 24;
const CUBE_NUM_TRI_INDICES: usize = 36;
const CUBE_NUM_QUAD_FACES: usize = 6;
const CUBE_NUM_TRI_FACES: usize = 12;

const HIT_LIST_LEN: usize = 16;
const COLORS: [[f32; 3]; 12] = [
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
];
const MODE: Mode = Mode::Normal;

const CUBE_VERTICES: Align16Array<[f32; 4], CUBE_NUM_VERTICES> = Align16Array([
    [-1.0, -1.0, -1.0, 0.0],
    [1.0, -1.0, -1.0, 0.0],
    [1.0, -1.0, 1.0, 0.0],
    [-1.0, -1.0, 1.0, 0.0],
    [-1.0, 1.0, -1.0, 0.0],
    [1.0, 1.0, -1.0, 0.0],
    [1.0, 1.0, 1.0, 0.0],
    [-1.0, 1.0, 1.0, 0.0],
]);

const CUBE_QUAD_INDICES: Align16Array<u32, CUBE_NUM_QUAD_INDICES> = Align16Array([
    0, 1, 3, 2, //
    5, 4, 6, 7, //
    0, 4, 5, 1, //
    6, 2, 3, 7, //
    0, 2, 6, 4, //
    3, 1, 5, 7, //
]);

const CUBE_TRI_INDICES: Align16Array<u32, CUBE_NUM_TRI_INDICES> = Align16Array([
    0, 1, 3, //
    3, 1, 2, //
    5, 4, 6, //
    6, 4, 7, //
    0, 4, 5, //
    5, 1, 0, //
    6, 2, 3, //
    3, 7, 6, //
    0, 2, 6, //
    6, 4, 0, //
    3, 1, 5, //
    5, 7, 3, //
]);

const CUBE_QUAD_FACES: [u32; CUBE_NUM_QUAD_FACES] = [4; 6];

// Extended ray structure that includes total transparency along the ray.
struct Ray2 {
    ray_hit: RayHit,
    transparency: f32, // accumulated transparency
    first_hit: u32,    // index of first hit
    last_hit: u32,     // index of last hit
    hit_geom_ids: [u32; HIT_LIST_LEN],
    hit_prim_ids: [u32; HIT_LIST_LEN],
}

impl Ray2 {
    fn new(ray: Ray) -> Self {
        Self {
            ray_hit: RayHit::new(ray),
            transparency: 1.0,
            first_hit: 0,
            last_hit: 0,
            hit_geom_ids: [0; HIT_LIST_LEN],
            hit_prim_ids: [0; HIT_LIST_LEN],
        }
    }
}

fn transparency_function(h: Vec3) -> f32 {
    let v = ((4.0 * h.x).sin() * (4.0 * h.y).cos() * (4.0 * h.z).sin()).abs();
    ((v - 0.1) * 3.0).clamp(0.0, 1.0)
}

struct IntersectContextExt {
    context: IntersectContext,
    ray_ext: Ray2,
}

fn render_pixel(x: u32, y: u32, camera: &Camera, scene: &Scene) -> u32 {
    let mut weight = 1.0;
    let mut color = Vec3::ZERO;
    let mut ctx = IntersectContextExt {
        context: IntersectContext::coherent(),
        ray_ext: Ray2 {
            ray_hit: RayHit::new(Ray::new_with_id(
                camera.pos.into(),
                camera.ray_dir((x as f32 + 0.5, y as f32 + 0.5)).into(),
                0, // needs to encode rayID for filter function
            )),
            transparency: 1.0,
            first_hit: 0,
            last_hit: 0,
            hit_geom_ids: [0; HIT_LIST_LEN],
            hit_prim_ids: [0; HIT_LIST_LEN],
        },
    };

    loop {
        scene.intersect(&mut ctx.context, &mut ctx.ray_ext.ray_hit);
        if !ctx.ray_ext.ray_hit.is_valid() {
            break;
        }

        let opacity = 1.0 - ctx.ray_ext.transparency;
        let diffuse = Vec3::from(COLORS[ctx.ray_ext.ray_hit.hit.primID as usize]);
        let la = diffuse * 0.5;
        color += weight * opacity * la;
        let light_dir = vec3(0.57, 0.57, 0.57);

        // initialize shadow ray
        let mut shadow_ray = Ray2::new(Ray::segment(
            ctx.ray_ext.ray_hit.hit_point(),
            light_dir.into(),
            0.001,
            f32::INFINITY,
        ));
        ctx.ray_ext = shadow_ray;

        // if !scene.occluded(&mut ctx.context, &mut ctx.ray_ext.ray_hit.ray) {
        //     let ll = diffuse
        //         * ctx.ray_ext.transparency
        //         * light_dir .dot(ctx.ray_ext.ray_hit.hit.normal_normalized().into())
        //           .clamp(0.0, 1.0);
        //     color += weight * opacity * ll;
        // }

        weight *= ctx.ray_ext.transparency;
        ctx.ray_ext.ray_hit.ray.tnear = 1.001 * ctx.ray_ext.ray_hit.ray.tfar;
        ctx.ray_ext.ray_hit.ray.tfar = f32::INFINITY;
        ctx.ray_ext.ray_hit.hit.geomID = embree::INVALID_ID;
        ctx.ray_ext.ray_hit.hit.primID = embree::INVALID_ID;
        ctx.ray_ext.transparency = 0.0;
    }

    rgba_to_u32(
        (color.x.clamp(0.0, 1.0) * 255.0) as u8,
        (color.y.clamp(0.0, 1.0) * 255.0) as u8,
        (color.z.clamp(0.0, 1.0) * 255.0) as u8,
        255,
    )
}

fn render_tile(tile: &mut Tile, camera: &Camera, scene: &Scene) {
    tile.pixels.iter_mut().enumerate().for_each(|(i, pixel)| {
        let x = tile.x + (i % tile.w as usize) as u32;
        let y = tile.y + (i / tile.w as usize) as u32;
        *pixel = render_pixel(x, y, camera, scene);
    });
}

fn render_tile_stream(tile: &mut Tile, camera: &Camera, scene: &Scene) { todo!() }

fn render_frame(tiled: &mut TiledImage, frame: &mut RgbaImage, camera: &Camera, scene: &Scene) {
    tiled.reset_pixels();
    match MODE {
        Mode::Normal => {
            tiled
                .par_tiles_mut()
                .for_each(|mut tile| render_tile(&mut tile, camera, scene));
        }
        Mode::Stream => {
            tiled
                .par_tiles_mut()
                .for_each(|mut tile| render_tile_stream(&mut tile, camera, scene));
        }
    }
    tiled.write_to_image(frame);
}

fn intersect_filter<'a>(
    rays: RayN<'a>,
    hits: HitN<'a>,
    mut valid: ValidMasks<'a>,
    ctx: &mut IntersectContext,
    _user_data: Option<&mut ()>,
) {
    assert_eq!(rays.len(), 1);

    let context = unsafe {
        let ctx = ctx as *mut IntersectContext as *mut IntersectContextExt;
        assert!(!ctx.is_null());
        &mut *ctx
    };

    // ignore invalid rays
    if valid[0] != -1 {
        return;
    }

    // calculate transparency
    let h = Vec3::from(rays.org(0)) + Vec3::from(rays.dir(0)) * rays.tfar(0);
    let t = transparency_function(h);

    // ignore hit if completely transparent
    if t >= 1.0 {
        valid[0] = 0;
    } else {
        // otherwise accept hit and remember transparency
        context.ray_ext.transparency = t;
    }
}

fn occlusion_filter<'a>(
    rays: RayN<'a>,
    hits: HitN<'a>,
    mut valid: ValidMasks<'a>,
    context: &mut IntersectContext,
    _user_data: Option<&mut ()>,
) {
    assert_eq!(rays.len(), 1);
    let context = unsafe {
        let ctx = context as *mut IntersectContext as *mut IntersectContextExt;
        assert!(!ctx.is_null());
        &mut *ctx
    };

    if valid[0] != -1 {
        return;
    }

    for i in context.ray_ext.first_hit..context.ray_ext.last_hit {
        let slot = i as usize % HIT_LIST_LEN;
        if context.ray_ext.hit_geom_ids[slot] == hits.geom_id(0)
            && context.ray_ext.hit_prim_ids[slot] == hits.prim_id(0)
        {
            valid[0] = 0; // ignore duplicate intersections
            return;
        }
    }

    // store hit in hit list
    let slot = context.ray_ext.last_hit % HIT_LIST_LEN as u32;
    context.ray_ext.hit_geom_ids[slot as usize] = hits.geom_id(0);
    context.ray_ext.hit_prim_ids[slot as usize] = hits.prim_id(0);
    context.ray_ext.last_hit += 1;

    eprintln!("{} {}", context.ray_ext.first_hit, context.ray_ext.last_hit);

    if context.ray_ext.last_hit - context.ray_ext.first_hit > HIT_LIST_LEN as u32 {
        context.ray_ext.first_hit += 1;
    }

    let h = Vec3::from(rays.org(0)) + Vec3::from(rays.dir(0)) * rays.tfar(0);

    let t = transparency_function(h);
    context.ray_ext.transparency *= t;
    if t != 0.0 {
        valid[0] = 0;
    }
}

fn create_ground_plane<'a>(device: &Device) -> Geometry<'a> {
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

fn create_cube<'a>(device: &Device, offset: Vec3, scale: Vec3, rotation: f32) -> Geometry<'a> {
    // create a triangulated cube with 12 triangles and 8 vertices
    let mut geom = device.create_geometry(GeometryKind::TRIANGLE).unwrap();
    let rotated = CUBE_VERTICES.map(|v| {
        let vtx = Vec3::new(v[0], v[1], v[2]);
        let vtx = offset + Mat3::from_axis_angle(Vec3::Y, rotation) * scale * vtx;
        [vtx.x, vtx.y, vtx.z, 0.0]
    });
    geom.set_new_buffer(BufferUsage::VERTEX, 0, Format::FLOAT3, 16, 8)
        .unwrap()
        .view_mut::<[f32; 4]>()
        .unwrap()
        .copy_from_slice(&rotated);
    geom.set_buffer(
        BufferUsage::INDEX,
        0,
        Format::UINT3,
        BufferSlice::from_slice(CUBE_TRI_INDICES.as_slice(), ..),
        std::mem::size_of::<u32>() * 3,
        CUBE_NUM_TRI_FACES,
    )
    .unwrap();

    // set intersection filter for the cube
    match MODE {
        Mode::Normal => {
            geom.set_intersect_filter_function(intersect_filter);
            geom.set_occluded_filter_function(occlusion_filter);
        }
        Mode::Stream => {
            // geom.set_intersect_filter_function(|_, _, _, _| {});
            // geom.set_occluded_filter_function(|_, _, _| {});
            todo!()
        }
    }
    geom.commit();
    geom
}

fn main() {
    let device = Device::new().unwrap();
    let mut scene = device.create_scene().unwrap();
    scene.set_build_quality(BuildQuality::HIGH);
    let ground = create_ground_plane(&device);
    let cube = create_cube(&device, vec3(0.0, 0.0, 0.0), vec3(10.0, 1.0, 1.0), 45.0);
    let ground_id = scene.attach_geometry(&ground);
    let cube_id = scene.attach_geometry(&cube);
    scene.commit();

    let display = support::Display::new(
        DEFAULT_DISPLAY_WIDTH,
        DEFAULT_DISPLAY_HEIGHT,
        "Intersection Filter",
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

        render_frame(&mut tiled, image, &camera, &scene);

        let elapsed = time - last_time;
        last_time = time;
        let fps = 1.0 / elapsed;
        eprint!("\r{} fps", fps);
    });
}
