#![allow(dead_code)]

extern crate embree;
extern crate support;

use embree::{
    BufferSlice, BufferUsage, Device, Format, IntersectContext, QuadMesh, Ray, TriangleMesh,
};
use support::Camera;

fn make_cube(device: &Device, vertex_colors: &[[f32; 3]]) -> TriangleMesh {
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

fn make_ground_plane(device: &Device) -> QuadMesh {
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

    let face_colors = vec![
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

    let cube = make_cube(&device, &vertex_colors);
    let ground = make_ground_plane(&device);

    let mut scene = device.create_scene().unwrap();
    let _ = scene.attach_geometry(&cube);
    let ground_id = scene.attach_geometry(&ground);
    scene.commit();

    let mut intersection_ctx = IntersectContext::coherent();

    let light_dir = glam::vec3(1.0, 1.0, 1.0).normalize();

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
        // Render the scene
        for j in 0..img_dims.1 {
            for i in 0..img_dims.0 {
                let dir = camera.ray_dir((i as f32 + 0.5, j as f32 + 0.5));
                let ray_hit = scene.intersect(
                    &mut intersection_ctx,
                    Ray::new(camera.pos.into(), dir.into()),
                );
                if ray_hit.is_valid() {
                    let p = image.get_pixel_mut(i, j);
                    let diffuse = if ray_hit.hit.geomID == ground_id {
                        glam::vec3(0.6, 0.6, 0.6)
                    } else {
                        glam::Vec3::from(face_colors[ray_hit.hit.primID as usize])
                    };

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
    });
}
