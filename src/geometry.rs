use sys::*;

use triangle_mesh;
use quad_mesh;
use instance;
use flat_linear_curve; 

pub enum Geometry<'a> {
    Triangle(triangle_mesh::TriangleMesh<'a>),
    Quad(quad_mesh::QuadMesh<'a>),
    Instance(instance::Instance<'a>),
    FlatLinearCurve(flat_linear_curve::FlatLinearCurve<'a>)
}

/// Geometry trait implemented by all Embree Geometry types
impl<'a> Geometry<'a> {
    pub fn handle(&self) -> RTCGeometry {
        match self {
            &Geometry::Triangle(ref m) => m.handle,
            &Geometry::Quad(ref q) => q.handle,
            &Geometry::Instance(ref i) => i.handle,
            &Geometry::FlatLinearCurve(ref c) => c.handle
        }
    }
    pub fn commit(&mut self) {
        unsafe {
            rtcCommitGeometry(self.handle());
        }
    }
}

impl<'a> Drop for Geometry<'a> {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseGeometry(self.handle());
        }
    }
}

impl<'a> PartialEq<Geometry<'a>> for Geometry<'a> {
    fn eq(&self, other: &Geometry) -> bool {
        self.handle() == other.handle()
    }
}

impl<'a> Eq for Geometry<'a> {}

