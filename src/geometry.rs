use sys::*;

use bezier_curve;
use bspline_curve;
use catmull_rom_curve;
use hermite_curve;
use instance;
use linear_curve;
use quad_mesh;
use triangle_mesh;

pub enum Geometry<'a> {
    Triangle(triangle_mesh::TriangleMesh<'a>),
    Quad(quad_mesh::QuadMesh<'a>),
    Instance(instance::Instance<'a>),
    LinearCurve(linear_curve::LinearCurve<'a>),
    BsplineCurve(bspline_curve::BsplineCurve<'a>),
    BezierCurve(bezier_curve::BezierCurve<'a>),
    HermiteCurve(hermite_curve::HermiteCurve<'a>),
    CatmullRomCurve(catmull_rom_curve::CatmullRomCurve<'a>),
}

/// Geometry trait implemented by all Embree Geometry types
impl<'a> Geometry<'a> {
    pub fn handle(&self) -> RTCGeometry {
        match self {
            &Geometry::Triangle(ref m) => m.handle,
            &Geometry::Quad(ref q) => q.handle,
            &Geometry::Instance(ref i) => i.handle,
            &Geometry::LinearCurve(ref lc) => lc.handle,
            &Geometry::BsplineCurve(ref bsc) => bsc.handle,
            &Geometry::BezierCurve(ref bzc) => bzc.handle,
            &Geometry::HermiteCurve(ref hc) => hc.handle,
            &Geometry::CatmullRomCurve(ref crc) => crc.handle,
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
