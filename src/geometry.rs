use sys::*;

/// Geometry trait implemented by all Embree Geometry types
pub trait Geometry {
    fn handle(&self) -> RTCGeometry;
    fn commit(&mut self) {
        unsafe {
            rtcCommitGeometry(self.handle());
        }
    }
}

impl PartialEq<Geometry> for Geometry {
    fn eq(&self, other: &Geometry) -> bool {
        self.handle() == other.handle()
    }
}

impl Eq for Geometry {}
