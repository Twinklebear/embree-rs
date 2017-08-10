use std::os::raw::c_uint;
use std::mem;

use sys::*;
use scene::Scene;
use ::GeometryFlags;

pub struct TriangleMesh<'a> {
    // TODO: This is fine with the same lifetime bound right?
    // The device lifetime parameterized by the Scene will be at
    // least as long as the Scene's lifetime.
    scene: &'a Scene<'a>,
    handle: c_uint,
    // TODO: Buffers
}
impl<'a> TriangleMesh<'a> {
    pub fn unanimated(scene: &'a Scene, flags: GeometryFlags,
                      num_tris: usize, num_verts: usize) -> TriangleMesh<'a>
    {
        let h = unsafe {
            rtcNewTriangleMesh(*scene.handle.borrow_mut(),
                               mem::transmute(flags), num_tris, num_verts, 1)
        };
        TriangleMesh { scene: scene, handle: h }
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

