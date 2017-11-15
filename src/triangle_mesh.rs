use std::os::raw::c_uint;
use std::mem;

use cgmath::{Vector3, Vector4};

use sys::*;
use ::{Scene, GeometryFlags, Buffer, BufferType};

pub struct TriangleMesh<'a> {
    scene: &'a Scene<'a>,
    handle: c_uint,
    pub vertex_buffer: Buffer<'a, Vector4<f32>>,
    pub index_buffer: Buffer<'a, Vector3<i32>>,
}
impl<'a> TriangleMesh<'a> {
    // TODO: It's awkward to be borrowing the scene as immutable all the
    // time when we're actually doing mutations on the scene data
    pub fn unanimated(scene: &'a Scene, flags: GeometryFlags,
                      num_tris: usize, num_verts: usize) -> TriangleMesh<'a>
    {
        let h = unsafe {
            rtcNewTriangleMesh(*scene.handle.borrow_mut(),
                               mem::transmute(flags), num_tris, num_verts, 1)
        };
        TriangleMesh {
            scene: scene,
            handle: h,
            vertex_buffer: Buffer::new(scene, h, num_verts, BufferType::VertexBuffer),
            index_buffer: Buffer::new(scene, h, num_tris, BufferType::IndexBuffer)
        }
    }
}

impl<'a> Drop for TriangleMesh<'a> {
    fn drop(&mut self) {
        unsafe {
            // TODO: Is borrowing mut here going to lead to tricky runtime issues?
            // Drops on a single thread won't occur in parallel right?
            rtcDeleteGeometry(*self.scene.handle.borrow_mut(), self.handle);
        }
    }
}

