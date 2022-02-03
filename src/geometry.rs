use crate::sys::*;

/// Geometry trait implemented by all Embree Geometry types
pub trait Geometry {
    fn handle(&self) -> RTCGeometry;
    fn commit(&mut self) {
        unsafe {
            rtcCommitGeometry(self.handle());
        }
    }
}
