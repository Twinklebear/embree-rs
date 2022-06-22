use cgmath::InnerSpace;
use std::f32;
use vec_max;
use vec_min;
use Vector3;

#[derive(Clone, Debug)]
pub struct AABB {
    pub p_min: Vector3,
    pub p_max: Vector3,
}

impl Default for AABB {
    fn default() -> Self {
        Self {
            p_min: Vector3::new(std::f32::MAX, std::f32::MAX, std::f32::MAX),
            p_max: Vector3::new(std::f32::MIN, std::f32::MIN, std::f32::MIN),
        }
    }
}

impl AABB {
    pub fn is_valid(&self) -> bool {
        self.p_max.x >= self.p_min.x && self.p_max.y >= self.p_min.y && self.p_max.z >= self.p_min.z
    }

    pub fn union_aabb(&self, b: &AABB) -> AABB {
        AABB {
            p_min: vec_min(&self.p_min, &b.p_min),
            p_max: vec_max(&self.p_max, &b.p_max),
        }
    }

    pub fn union_vec(&self, v: &Vector3) -> AABB {
        AABB {
            p_min: vec_min(&self.p_min, v),
            p_max: vec_max(&self.p_max, v),
        }
    }

    #[inline]
    pub fn size(&self) -> Vector3 {
        self.p_max - self.p_min
    }

    #[inline]
    pub fn center(&self) -> Vector3 {
        self.size() * 0.5 + self.p_min
    }
}
