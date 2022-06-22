#![allow(dead_code)]

extern crate cgmath;
extern crate embree;
extern crate rand;
extern crate rayon;
extern crate support;
extern crate tobj;
use std::path::Path;

use cgmath::{InnerSpace, Matrix3, Point2, Vector2, Vector3, Vector4};
use embree::{Device, Geometry, IntersectContext, Ray, RayHit, Scene, TriangleMesh};
use rand::prelude::*;
use rayon::prelude::*;
use support::{Camera, AABB};

/// Function to sample a point inside a 2D disk
pub fn concentric_sample_disk(u: Point2<f32>) -> Point2<f32> {
    // map uniform random numbers to $[-1,1]^2$
    let u_offset: Point2<f32> = u * 2.0 as f32 - Vector2 { x: 1.0, y: 1.0 };
    // handle degeneracy at the origin
    if u_offset.x == 0.0 as f32 && u_offset.y == 0.0 as f32 {
        return Point2 { x: 0.0, y: 0.0 };
    }
    // apply concentric mapping to point
    let theta: f32;
    let r: f32;
    if u_offset.x.abs() > u_offset.y.abs() {
        r = u_offset.x;
        theta = std::f32::consts::FRAC_PI_4 * (u_offset.y / u_offset.x);
    } else {
        r = u_offset.y;
        theta =
            std::f32::consts::FRAC_PI_2 - std::f32::consts::FRAC_PI_4 * (u_offset.x / u_offset.y);
    }
    Point2 {
        x: theta.cos(),
        y: theta.sin(),
    } * r
}
/// Function to sample cosine-weighted hemisphere
pub fn cosine_sample_hemisphere(u: Point2<f32>) -> Vector3<f32> {
    let d: Point2<f32> = concentric_sample_disk(u);
    let z: f32 = (0.0 as f32).max(1.0 as f32 - d.x * d.x - d.y * d.y).sqrt();
    Vector3 { x: d.x, y: d.y, z }
}

// "Building an Orthonormal Basis, Revisited" by Duff et al., JCGT, 2017
// http://jcgt.org/published/0006/01/01/
pub struct Frame(Matrix3<f32>);
impl Frame {
    pub fn new(n: Vector3<f32>) -> Frame {
        let sign = n.z.signum();
        let a = -1.0 / (sign + n.z);
        let b = n.x * n.y * a;
        Frame {
            0: Matrix3 {
                x: Vector3::new(1.0 + sign * n.x * n.x * a, sign * b, -sign * n.x),
                y: Vector3::new(b, sign + n.y * n.y * a, -n.y),
                z: n,
            },
        }
    }

    pub fn to_world(&self, v: Vector3<f32>) -> Vector3<f32> {
        self.0.x * v.x + self.0.y * v.y + self.0.z * v.z
    }

    pub fn to_local(&self, v: Vector3<f32>) -> Vector3<f32> {
        Vector3::new(v.dot(self.0.x), v.dot(self.0.y), v.dot(self.0.z))
    }
}

// It is an example of a custom structure
// that encapsulate the embree commited scene
pub struct AOIntegrator<'embree> {
    // Camera (can be updated using update_cam method)
    camera: Camera,
    // A borrowed commited scene
    // Note here the lifetime for borrowing and devide are the same
    // Which is fine in our case
    rtscene: &'embree embree::CommittedScene<'embree>,
    // List of models loaded from tobj
    models: Vec<tobj::Model>,
    // Meshs ids (to map embree intersection with the models list)
    mesh_ids: Vec<u32>,
    // Max distance (to compute the AO)
    max_distance: Option<f32>,
}

impl<'embree> AOIntegrator<'embree> {
    // Update the camera parameter
    // return true if the camera paramter get changed
    pub fn update_cam(&mut self, camera: Camera) -> bool {
        if self.camera == camera {
            false
        } else {
            self.camera = camera;
            true
        }
    }

    // Simple AO computation method
    pub fn render(&self, i: u32, j: u32, u: Point2<f32>) -> f32 {
        let dir = self.camera.ray_dir((i as f32 + 0.5, j as f32 + 0.5));
        let ray = Ray::new(self.camera.pos, dir);
        let mut ray_hit = RayHit::new(ray);

        let mut intersection_ctx = IntersectContext::coherent();
        self.rtscene.intersect(&mut intersection_ctx, &mut ray_hit);

        if ray_hit.hit.hit() {
            let mesh = &self.models[self.mesh_ids[ray_hit.hit.geomID as usize] as usize].mesh;
            // Compute the normal at the intersection point
            let mut n = {
                if !mesh.normals.is_empty() {
                    // In this case, we will interpolate the normals
                    // Note that this operation is supported by embree (internal)
                    let prim = ray_hit.hit.primID as usize;
                    let tri = [
                        mesh.indices[prim * 3] as usize,
                        mesh.indices[prim * 3 + 1] as usize,
                        mesh.indices[prim * 3 + 2] as usize,
                    ];

                    // Retrive the different normal vectors
                    let na = Vector3::new(
                        mesh.normals[tri[0] * 3],
                        mesh.normals[tri[0] * 3 + 1],
                        mesh.normals[tri[0] * 3 + 2],
                    );

                    let nb = Vector3::new(
                        mesh.normals[tri[1] * 3],
                        mesh.normals[tri[1] * 3 + 1],
                        mesh.normals[tri[1] * 3 + 2],
                    );

                    let nc = Vector3::new(
                        mesh.normals[tri[2] * 3],
                        mesh.normals[tri[2] * 3 + 1],
                        mesh.normals[tri[2] * 3 + 2],
                    );

                    // Interpolate
                    let w = 1.0 - ray_hit.hit.u - ray_hit.hit.v;
                    (na * w + nb * ray_hit.hit.u + nc * ray_hit.hit.v).normalize()
                } else {
                    // As the mesh normal is not provided
                    // we will uses the geometric normals
                    // fortunately, embree computes this information for us
                    Vector3::new(ray_hit.hit.Ng_x, ray_hit.hit.Ng_y, ray_hit.hit.Ng_z).normalize()
                }
            };

            // We flip the normal automatically in this case
            if n.dot(dir) > 0.0 {
                n *= -1.0;
            }

            // Create local frame
            let frame = Frame::new(n);
            let p = self.camera.pos + dir * ray_hit.ray.tfar;

            // Do cosine weighted sampling of the outgoing direction
            // note that we will not evaluate the cosine term from this point
            // as it get canceled by the PDF
            let dir = frame.to_world(cosine_sample_hemisphere(u));

            // Launch a second ray from the intersection point
            let ray = Ray::new(p, dir);
            let mut ray_hit = RayHit::new(ray);
            ray_hit.ray.tnear = 0.00001; // Avoid self intersection
            let mut intersection_ctx = IntersectContext::incoherent();
            self.rtscene.intersect(&mut intersection_ctx, &mut ray_hit);
            if ray_hit.hit.hit() {
                match self.max_distance {
                    None => 0.0,
                    Some(t) => {
                        if ray_hit.ray.tfar > t {
                            1.0
                        } else {
                            0.0
                        }
                    }
                }
            } else {
                1.0
            }
        } else {
            0.0
        }
    }
}

fn main() {
    let mut display = support::Display::new(512, 512, "OBJ AO Viewer");
    let device = Device::new();

    // Expect <obj_path> [max_distance]
    let args: Vec<_> = std::env::args().collect();

    // Get the distance. If nothing provided
    // use infinity
    let max_distance = match args.len() {
        1 => panic!("Need to provide obj path argument"),
        2 => None,
        3 => {
            let d = args[2]
                .parse::<f32>()
                .expect("Impossible to parse the max distance: need to be float");
            if d <= 0.0 {
                panic!(format!("Max distance need to be more than 0.0 ({})", d));
            }
            Some(d)
        }
        _ => panic!(
            "Too much arguments provided. Only supporting obj path and max distance arguments"
        ),
    };

    // Load the obj
    let (models, _) = tobj::load_obj(&Path::new(&args[1])).unwrap();
    let mut tri_geoms = Vec::new();
    let mut aabb = AABB::default();
    for m in models.iter() {
        let mesh = &m.mesh;
        println!(
            "Mesh has {} triangles and {} verts",
            mesh.indices.len() / 3,
            mesh.positions.len() / 3
        );

        let mut tris =
            TriangleMesh::unanimated(&device, mesh.indices.len() / 3, mesh.positions.len() / 3);
        {
            let mut verts = tris.vertex_buffer.map();
            let mut tris = tris.index_buffer.map();
            for i in 0..mesh.positions.len() / 3 {
                aabb = aabb.union_vec(&Vector3::new(
                    mesh.positions[i * 3],
                    mesh.positions[i * 3 + 1],
                    mesh.positions[i * 3 + 2],
                ));
                verts[i] = Vector4::new(
                    mesh.positions[i * 3],
                    mesh.positions[i * 3 + 1],
                    mesh.positions[i * 3 + 2],
                    0.0,
                );
            }

            for i in 0..mesh.indices.len() / 3 {
                tris[i] = Vector3::new(
                    mesh.indices[i * 3],
                    mesh.indices[i * 3 + 1],
                    mesh.indices[i * 3 + 2],
                );
            }
        }
        let mut tri_geom = Geometry::Triangle(tris);
        tri_geom.commit();
        tri_geoms.push(tri_geom);
    }
    display = display.aabb(aabb);

    println!("Commit the scene ... ");
    let mut scene = Scene::new(&device);
    let mut mesh_ids = Vec::with_capacity(models.len());
    for g in tri_geoms.drain(0..) {
        let id = scene.attach_geometry(g);
        mesh_ids.push(id);
    }
    let rtscene = scene.commit();

    // Create my custom object that will compute the ambiant occlusion
    let mut scene = AOIntegrator {
        models,
        mesh_ids,
        camera: Camera::look_at(
            Vector3::new(-1.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            55.0,
            (512, 512),
        ),
        rtscene: &rtscene,
        max_distance,
    };

    // Variables to average AO computation across frames
    let mut spp = 0;
    let mut img = Vec::new();

    println!("Rendering launched ... ");
    display.run(|image, camera_pose, _| {
        for p in image.iter_mut() {
            *p = 0;
        }
        let img_dims = image.dimensions();

        // Update the camera
        if scene.update_cam(Camera::look_dir(
            camera_pose.pos,
            camera_pose.dir,
            camera_pose.up,
            55.0,
            img_dims,
        )) {
            // If the camera have moved, we clear previous accumulated results.
            spp = 0;
            img.resize((img_dims.0 * img_dims.1) as usize, 0.0);
            for i in &mut img {
                (*i) = 0.0;
            }
        }

        // Render the scene with Rayon. Here each pixel compute 1 spp AO
        img.par_chunks_mut(image.width() as usize)
            .enumerate()
            .for_each(|(y, row)| {
                let mut rng = rand::thread_rng();
                for (x, p) in row.iter_mut().enumerate() {
                    let u = Point2::new(rng.gen(), rng.gen());
                    // Weighting average
                    (*p) =
                        (*p * spp as f32 + scene.render(x as u32, y as u32, u)) / (spp + 1) as f32;
                }
            });
        spp += 1;

        // Copy the accumulated result inside the image buffer
        let raw_out = image.as_mut();
        raw_out.chunks_mut(3).zip(img.iter()).for_each(|(p, v)| {
            p[0] = (v * 255.0) as u8;
            p[1] = (v * 255.0) as u8;
            p[2] = (v * 255.0) as u8;
        });
    });
}
