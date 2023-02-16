//! This example shows how to intersect a ray with a single triangle.

use embree::{BufferUsage, Device, Format, GeometryType, IntersectContext, Ray, Scene};

/// Casts a single ray with the given origin and direction.
fn cast_ray(scene: &Scene, origin: [f32; 3], direction: [f32; 3]) {
    let ray = Ray::new(origin, direction);

    // The intersect context can be used to set intersection filters or flags, and
    // it also contains the instance ID stack used in multi-level instancing.
    let mut context = IntersectContext::coherent();

    // Intersect a single ray with the scene.
    let ray_hit = scene.intersect(&mut context, ray);

    print!("{origin:?} ");

    if ray_hit.is_valid() {
        println!(
            "Found intersection on geometry {}, primitive {} at tfar = {}",
            ray_hit.hit.geomID, ray_hit.hit.primID, ray_hit.ray.tfar
        );
    } else {
        println!("Did not find any intersection");
    }
}

fn main() {
    let device = Device::new().unwrap();

    device.set_error_function(|err, msg| {
        println!("error {:?}: {}", err, msg);
    });

    let mut scene = device.create_scene().unwrap();
    {
        // Create a triangle mesh geometry, and initialise a single triangle.
        let mut triangle = device.create_geometry(GeometryType::TRIANGLE).unwrap();
        triangle
            .set_new_buffer(
                BufferUsage::VERTEX,
                0,
                Format::FLOAT3,
                3 * std::mem::size_of::<f32>(),
                3,
            )
            .unwrap()
            .view_mut::<[f32; 3]>()
            .unwrap()
            .copy_from_slice(&[
                [0.0, 0.0, 0.0], // vertex 0
                [1.0, 0.0, 0.0], // vertex 1
                [0.0, 1.0, 0.0], // vertex 2
            ]);
        triangle
            .set_new_buffer(
                BufferUsage::INDEX,
                0,
                Format::UINT3,
                3 * std::mem::size_of::<u32>(),
                1,
            )
            .unwrap()
            .view_mut::<[u32; 3]>()
            .unwrap()
            .copy_from_slice(&[
                [0, 1, 2], // triangle 0
            ]);

        // Geometry objects must be committed when you are done setting them up,
        // otherwise you will not get any intersection results.
        triangle.commit();

        scene.attach_geometry(&triangle);

        // The scene must also be committed when you are done setting it up.
        scene.commit();

        // The geometry will be dropped when it goes out of scope, but the scene
        // will still hold a reference to it.
    }

    cast_ray(&scene, [0.0, 0.0, -1.0], [0.0, 0.0, 1.0]);
    cast_ray(&scene, [1.0, 1.0, -1.0], [0.0, 0.0, 1.0]);
}
