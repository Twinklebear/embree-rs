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
    AlignedArray, BufferSlice, BufferUsage, BuildQuality, Device, Format, Geometry, GeometryKind,
    HitN, IntersectContextExt, Ray, RayHit, RayN, Scene, SoAHit, SoARay, ValidityN, INVALID_ID,
};
use glam::{vec3, Mat4, Vec3, Vec4};
use support::{
    rgba_to_u32, Camera, DebugState, Mode, ParallelIterator, TileMut, TiledImage,
    DEFAULT_DISPLAY_HEIGHT, DEFAULT_DISPLAY_WIDTH,
};

const CUBE_NUM_VERTICES: usize = 8;
const CUBE_NUM_QUAD_INDICES: usize = 24;
const CUBE_NUM_TRI_INDICES: usize = 36;
const CUBE_NUM_QUAD_FACES: usize = 6;
const CUBE_NUM_TRI_FACES: usize = 12;

const MODE: Mode = Mode::Stream;

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

const CUBE_VERTICES: AlignedArray<[f32; 4], CUBE_NUM_VERTICES> = AlignedArray([
    [-1.0, -1.0, -1.0, 1.0],
    [-1.0, -1.0, 1.0, 1.0],
    [-1.0, 1.0, -1.0, 1.0],
    [-1.0, 1.0, 1.0, 1.0],
    [1.0, -1.0, -1.0, 1.0],
    [1.0, -1.0, 1.0, 1.0],
    [1.0, 1.0, -1.0, 1.0],
    [1.0, 1.0, 1.0, 1.0],
]);

#[allow(dead_code)]
const CUBE_QUAD_INDICES: AlignedArray<u32, CUBE_NUM_QUAD_INDICES> = AlignedArray([
    0, 1, 3, 2, //
    5, 4, 6, 7, //
    0, 4, 5, 1, //
    6, 2, 3, 7, //
    0, 2, 6, 4, //
    3, 1, 5, 7, //
]);

const CUBE_TRI_INDICES: AlignedArray<u32, CUBE_NUM_TRI_INDICES> = AlignedArray([
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

#[allow(dead_code)]
const CUBE_QUAD_FACES: [u32; CUBE_NUM_QUAD_FACES] = [4; 6];

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct RayExtra {
    transparency: f32, // accumulated transparency
    first_hit: u32,    // index of first hit
    last_hit: u32,     // index of last hit
    hit_geom_ids: [u32; HIT_LIST_LEN],
    hit_prim_ids: [u32; HIT_LIST_LEN],
}

impl Default for RayExtra {
    fn default() -> Self {
        RayExtra {
            transparency: 1.0,
            first_hit: 0,
            last_hit: 0,
            hit_geom_ids: [0; HIT_LIST_LEN],
            hit_prim_ids: [0; HIT_LIST_LEN],
        }
    }
}

fn transparency_function(h: [f32; 3]) -> f32 {
    let v = ((4.0 * h[0]).sin() * (4.0 * h[1]).cos() * (4.0 * h[2]).sin()).abs();
    ((v - 0.1) * 3.0).clamp(0.0, 1.0)
}

type IntersectContext2 = IntersectContextExt<RayExtra>;
type IntersectContext2Stream = IntersectContextExt<Vec<RayExtra>>;

fn render_pixel(x: u32, y: u32, camera: &Camera, scene: &Scene) -> u32 {
    let mut weight = 1.0;
    let mut color = Vec3::ZERO;
    let mut primary = RayHit::from_ray(Ray::new(
        camera.pos.into(),
        camera.ray_dir((x as f32 + 0.5, y as f32 + 0.5)).into(),
        0.0,
        f32::INFINITY,
        0.0,
        u32::MAX,
        0, // needs to encode rayID for filter function
    ));
    let mut primary_extra = RayExtra {
        transparency: 0.0,
        ..Default::default()
    };
    let mut primary_ctx = IntersectContext2::coherent(primary_extra);

    let shadow_extra = RayExtra::default();
    let mut ctx_shadow = IntersectContext2::coherent(shadow_extra);

    loop {
        scene.intersect(&mut primary_ctx, &mut primary);

        if !primary.hit.is_valid() {
            break;
        }

        let opacity = 1.0 - primary_ctx.ext.transparency;
        let diffuse = Vec3::from(COLORS[primary.hit.primID as usize]);
        let la = diffuse * 0.5;
        color += weight * opacity * la;
        let light_dir = vec3(0.57, 0.57, 0.57);

        // initialize shadow ray
        let mut shadow_ray = Ray::segment_with_id(
            primary.ray.hit_point(),
            light_dir.into(),
            0.001,
            f32::INFINITY,
            0,
        );

        if !scene.occluded(&mut ctx_shadow, &mut shadow_ray) {
            let ll = diffuse
                * shadow_extra.transparency
                * light_dir
                    .dot(primary.hit.unit_normal().into()) //
                    .clamp(0.0, 1.0);
            color += weight * opacity * ll;
        }

        weight *= primary_extra.transparency;
        primary.ray.tnear = 1.001 * primary.ray.tfar;
        primary.ray.tfar = f32::INFINITY;
        primary.hit.geomID = embree::INVALID_ID;
        primary.hit.primID = embree::INVALID_ID;
        primary_extra.transparency = 0.0;
    }

    rgba_to_u32(
        (color.x.clamp(0.0, 1.0) * 255.0) as u8,
        (color.y.clamp(0.0, 1.0) * 255.0) as u8,
        (color.z.clamp(0.0, 1.0) * 255.0) as u8,
        255,
    )
}

fn render_tile(tile: &mut TileMut, camera: &Camera, scene: &Scene) {
    tile.pixels.iter_mut().enumerate().for_each(|(i, pixel)| {
        let x = tile.x + (i % tile.w as usize) as u32;
        let y = tile.y + (i / tile.w as usize) as u32;
        *pixel = render_pixel(x, y, camera, scene);
    });
}

fn render_tile_stream(tile: &mut TileMut, width: u32, height: u32, camera: &Camera, scene: &Scene) {
    let tile_x_end = (tile.x + tile.w).min(width);
    let tile_y_end = (tile.y + tile.h).min(height);
    let tile_w = tile_x_end - tile.x;
    let tile_h = tile_y_end - tile.y;
    let tile_size = (tile_w * tile_h) as usize;
    let mut weights = vec![1.0; tile_size];
    let mut colors = vec![Vec3::ZERO; tile_size];
    let mut primary = vec![RayHit::default(); tile_size];
    let primary_extra = vec![RayExtra::default(); tile_size];
    let mut primary_ctx = IntersectContext2Stream::coherent(primary_extra);
    let mut shadows = vec![Ray::default(); tile_size];
    let shadows_extra = vec![RayExtra::default(); tile_size];
    let mut shadows_ctx = IntersectContext2Stream::coherent(shadows_extra);
    let mut validates = vec![true; tile_size];

    // actual number of rays in stream may be less than number of pixels in tile
    let mut i = 0;
    let mut num_active = 0;
    // generate stream of primary rays
    for y in tile.y..tile_y_end {
        for x in tile.x..tile_x_end {
            num_active += 1;
            validates[i] = true;
            primary[i] = RayHit::from_ray(Ray::segment_with_id(
                camera.pos.into(),
                camera.ray_dir((x as f32 + 0.5, y as f32 + 0.5)).into(),
                0.0,
                f32::INFINITY,
                i as u32, // needs to encode rayID for filter function
            ));
            primary_ctx.ext[i] = RayExtra {
                transparency: 0.0,
                ..Default::default()
            };
            i += 1;
        }
    }

    let light_dir = vec3(0.57, 0.57, 0.57);

    while num_active > 0 {
        //let mut primary_context =
        // IntersectContext2Stream::coherent(primaries_extra.as_mut_slice());
        scene.intersect_stream_aos(&mut primary_ctx, &mut primary);
        // terminate rays and update color
        for n in 0..tile_size as usize {
            // invalidate shadow rays by default
            shadows[n].tnear = f32::INFINITY;
            shadows[n].tfar = f32::NEG_INFINITY;

            // ignore invalid rays
            if !validates[n] {
                continue;
            }

            // terminate ray if it did not hit anything
            if !primary[n].hit.is_valid() {
                validates[n] = false;
                continue;
            }

            // update color
            let opacity = 1.0 - primary_ctx.ext[n].transparency;
            let diffuse = Vec3::from(COLORS[primary[n].hit.primID as usize]);
            let la = diffuse * 0.5;
            colors[n] += weights[n] * opacity * la;

            // initialize shadow ray
            {
                shadows[n] = Ray::segment_with_id(
                    primary[n].ray.hit_point(),
                    light_dir.into(),
                    0.001,
                    f32::INFINITY,
                    n as u32,
                );
                shadows_ctx.ext[n] = RayExtra::default();
            }
        }

        // trace shadow rays
        // let mut shadow_context =
        // IntersectContext2Stream::coherent(shadows_extra.as_mut_slice());
        scene.occluded_stream_aos(&mut shadows_ctx, &mut shadows);

        // add light contribution and generate transmission rays
        num_active = 0;
        for n in 0..tile_size as usize {
            // invalidate rays by default
            let primary_tfar = primary[n].ray.tfar;
            primary[n].ray.tnear = f32::INFINITY;
            primary[n].ray.tfar = f32::NEG_INFINITY;

            /* ignore invalid rays */
            if !validates[n] {
                continue;
            }

            num_active += 1;

            // add light contribution
            let opacity = 1.0 - primary_ctx.ext[n].transparency;
            let diffuse = Vec3::from(COLORS[primary[n].hit.primID as usize]);
            if shadows[n].tfar != f32::NEG_INFINITY {
                let ll = diffuse
                    * shadows_ctx.ext[n].transparency
                    * light_dir
                        .dot(primary[n].hit.unit_normal().into())
                        .clamp(0.0, 1.0);
                colors[n] += weights[n] * opacity * ll;
            }
            /* initialize transmission ray */
            weights[n] *= primary_ctx.ext[n].transparency;
            primary[n].ray.tnear = 1.001 * primary_tfar;
            primary[n].ray.tfar = f32::INFINITY;
            primary[n].hit.geomID = INVALID_ID;
            primary[n].hit.primID = INVALID_ID;
            primary_ctx.ext[n].transparency = 0.0;
        }
    }

    // write color to tile
    i = 0;
    for y in 0..tile_h {
        for x in 0..tile_w {
            tile.pixels[(y * tile_w + x) as usize] = rgba_to_u32(
                (colors[i].x.clamp(0.0, 1.0) * 255.0) as u8,
                (colors[i].y.clamp(0.0, 1.0) * 255.0) as u8,
                (colors[i].z.clamp(0.0, 1.0) * 255.0) as u8,
                255,
            );
            i += 1;
        }
    }
}

fn render_frame(frame: &mut TiledImage, camera: &Camera, scene: &Scene) {
    let width = frame.width;
    let height = frame.height;
    match MODE {
        Mode::Normal => {
            frame
                .par_tiles_mut()
                .for_each(|mut tile| render_tile(&mut tile, camera, scene));
        }
        Mode::Stream => {
            frame
                .par_tiles_mut()
                .for_each(|mut tile| render_tile_stream(&mut tile, width, height, camera, scene));
        }
    }
}

fn intersect_filter<'a>(
    rays: RayN<'a>,
    _hits: HitN<'a>,
    mut valid: ValidityN<'a>,
    ctx: &mut IntersectContext2,
    _user_data: Option<&mut ()>,
) {
    assert_eq!(rays.len(), 1);

    // ignore invalid rays
    if valid[0] != -1 {
        return;
    }

    // calculate transparency
    let t = transparency_function(rays.hit_point(0));

    // ignore hit if completely transparent
    if t >= 1.0 {
        valid[0] = 0;
    } else {
        // otherwise accept hit and remember transparency
        ctx.ext.transparency = t;
    }
}

fn intersect_filter_n<'a, 'b>(
    rays: RayN<'a>,
    _hits: HitN<'a>,
    mut valid: ValidityN<'a>,
    ctx: &'b mut IntersectContext2Stream,
    _user_data: Option<&mut ()>,
) {
    assert_eq!(rays.len(), valid.len());
    let n = rays.len();
    // iterate over all rays in ray packet
    for i in 0..n {
        // calculate loop and execution mask
        let vi = i;
        if vi >= n {
            continue;
        }

        // ignore invalid rays
        if valid[vi] != -1 {
            continue;
        }

        // calculate transparency
        let t = transparency_function(rays.hit_point(i));
        // ignore hit if completely transparent
        if t >= 1.0 {
            valid[vi] = 0;
        } else {
            // otherwise accept hit and remember transparency
            ctx.ext[rays.id(i) as usize].transparency = t;
        }
    }
}

fn occluded_filter<'a>(
    rays: RayN<'a>,
    hits: HitN<'a>,
    mut valid: ValidityN<'a>,
    context: &mut IntersectContext2,
    _user_data: Option<&mut ()>,
) {
    assert_eq!(rays.len(), 1);

    if valid[0] != -1 {
        return;
    }

    for i in context.ext.first_hit..context.ext.last_hit {
        let slot = i as usize % HIT_LIST_LEN;
        if context.ext.hit_geom_ids[slot] == hits.geom_id(0)
            && context.ext.hit_prim_ids[slot] == hits.prim_id(0)
        {
            valid[0] = 0; // ignore duplicate intersections
            return;
        }
    }

    // store hit in hit list
    let slot = context.ext.last_hit % HIT_LIST_LEN as u32;
    context.ext.hit_geom_ids[slot as usize] = hits.geom_id(0);
    context.ext.hit_prim_ids[slot as usize] = hits.prim_id(0);
    context.ext.last_hit += 1;

    if context.ext.last_hit - context.ext.first_hit > HIT_LIST_LEN as u32 {
        context.ext.first_hit += 1;
    }

    let t = transparency_function(rays.hit_point(0));
    context.ext.transparency *= t;
    if t != 0.0 {
        valid[0] = 0;
    }
}

fn occluded_filter_n<'a>(
    rays: RayN<'a>,
    hits: HitN<'a>,
    mut valid: ValidityN<'a>,
    ctx: &mut IntersectContext2Stream,
    _user_data: Option<&mut ()>,
) {
    assert_eq!(rays.len(), valid.len());
    let n = rays.len();

    // iterate over all rays in ray packet
    for i in 0..n {
        // calculate loop and execution mask
        let vi = i as usize;
        if vi >= n {
            continue;
        }

        // ignore invalid rays
        if valid[vi] != -1 {
            continue;
        }

        let hit_geom_id = hits.geom_id(i);
        let hit_prim_id = hits.prim_id(i);

        // the occlusion filter may be called multiple times with the same hit,
        // we remember the last N hits, and skip duplicates
        let rid = rays.id(i) as usize;
        let first_hit = ctx.ext[rid].first_hit;
        let mut last_hit = ctx.ext[rid].last_hit;
        for j in first_hit..last_hit {
            let slot = j as usize % HIT_LIST_LEN;
            let last_geom_id = ctx.ext[rid].hit_geom_ids[slot];
            let last_prim_id = ctx.ext[rid].hit_prim_ids[slot];
            if last_geom_id == hit_geom_id && last_prim_id == hit_prim_id {
                valid[vi] = 0; // ignore duplicate intersections
                break;
            }
        }
        if valid[vi] == 0 {
            continue;
        }

        // store hit in hit list
        let slot = last_hit % HIT_LIST_LEN as u32;
        ctx.ext[rid].hit_geom_ids[slot as usize] = hit_geom_id;
        ctx.ext[rid].hit_prim_ids[slot as usize] = hit_prim_id;
        last_hit += 1;
        ctx.ext[rid].last_hit = last_hit;
        if last_hit - first_hit >= HIT_LIST_LEN as u32 {
            ctx.ext[rid].first_hit = first_hit + 1;
        }

        // calculate transparency
        let t = transparency_function(rays.hit_point(i)) * ctx.ext[rid].transparency;
        ctx.ext[rid].transparency = t;

        // reject a hit if not fully opaque
        if t != 0.0 {
            valid[vi] = 0;
        }
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
        (Mat4::from_translation(offset)
            * Mat4::from_axis_angle(Vec3::Y, rotation)
            * Mat4::from_scale(scale)
            * Vec4::from(v))
        .into()
    });
    geom.set_new_buffer(
        BufferUsage::VERTEX,
        0,
        Format::FLOAT3,
        16,
        CUBE_NUM_VERTICES,
    )
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
            geom.set_occluded_filter_function(occluded_filter);
        }
        Mode::Stream => {
            geom.set_intersect_filter_function(intersect_filter_n);
            geom.set_occluded_filter_function(occluded_filter_n);
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
    scene.attach_geometry(&ground);
    scene.attach_geometry(&cube);
    scene.commit();

    let display = support::Display::new(
        DEFAULT_DISPLAY_WIDTH,
        DEFAULT_DISPLAY_HEIGHT,
        "Intersection Filter",
    );

    let state = DebugState {
        scene: scene.clone(),
        user: (),
    };

    support::display::run(
        display,
        state,
        |_, _| {},
        move |image, camera, _, _| {
            render_frame(image, &camera, &scene);
        },
        |_| {},
    );
}
