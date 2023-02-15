use crate::{sys, INVALID_ID};

pub type Ray = sys::RTCRay;
pub type Hit = sys::RTCHit;
pub type RayHit = sys::RTCRayHit;

impl Ray {
    /// Create a new ray starting at `origin` and heading in direction `dir`
    pub fn new(origin: [f32; 3], dir: [f32; 3]) -> Ray {
        Ray::segment(origin, dir, 0.0, f32::INFINITY)
    }

    pub fn segment(origin: [f32; 3], dir: [f32; 3], tnear: f32, tfar: f32) -> Ray {
        sys::RTCRay {
            org_x: origin[0],
            org_y: origin[1],
            org_z: origin[2],
            dir_x: dir[0],
            dir_y: dir[1],
            dir_z: dir[2],
            tnear,
            tfar,
            time: 0.0,
            mask: u32::MAX,
            id: 0,
            flags: 0,
        }
    }
}

impl Default for Hit {
    fn default() -> Self {
        Hit {
            Ng_x: 0.0,
            Ng_y: 0.0,
            Ng_z: 0.0,
            u: 0.0,
            v: 0.0,
            primID: INVALID_ID,
            geomID: INVALID_ID,
            instID: [INVALID_ID; 1],
        }
    }
}

impl RayHit {
    pub fn new(ray: Ray) -> RayHit {
        sys::RTCRayHit {
            ray,
            hit: Hit::default(),
        }
    }

    /// Returns true if the hit is valid (i.e. the ray hit something).
    pub fn is_valid(&self) -> bool { self.hit.geomID != INVALID_ID }

    /// Returns the normal of the hit point (normalized).
    pub fn normal(&self) -> [f32; 3] {
        let len = (self.hit.Ng_x * self.hit.Ng_x
            + self.hit.Ng_y * self.hit.Ng_y
            + self.hit.Ng_z * self.hit.Ng_z)
            .sqrt();
        if len == 0.0 {
            [0.0, 0.0, 0.0]
        } else {
            let len = 1.0 / len;
            [
                self.hit.Ng_x * len,
                self.hit.Ng_y * len,
                self.hit.Ng_z * len,
            ]
        }
    }

    /// Returns the barycentric coordinates of the hit point.
    pub fn uv(&self) -> [f32; 2] { [self.hit.u, self.hit.v] }

    /// Returns the hit point.
    pub fn hit_point(&self) -> [f32; 3] {
        let t = self.ray.tfar;
        [
            self.ray.org_x + self.ray.dir_x * t,
            self.ray.org_y + self.ray.dir_y * t,
            self.ray.org_z + self.ray.dir_z * t,
        ]
    }
}
