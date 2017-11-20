use std::cmp::{PartialEq, Eq};

/// Geometry trait implemented by all Embree Geometry types
pub trait Geometry {
    fn geom_id(&self) -> u32;
}

impl PartialEq<Geometry> for Geometry {
    fn eq(&self, other: &Geometry) -> bool {
        self.geom_id() == other.geom_id()
    }
}

impl PartialEq<u32> for Geometry {
    fn eq(&self, other: &u32) -> bool {
        self.geom_id() == *other
    }
}

impl PartialEq<Geometry> for u32 {
    fn eq(&self, other: &Geometry) -> bool {
        *self == other.geom_id()
    }
}

impl Eq for Geometry {}

