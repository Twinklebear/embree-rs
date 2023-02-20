use crate::{sys, SoAHit, SoAHitIter, SoAHitRef, SoARay, SoARayIter, SoARayIterMut};

pub type Ray4 = sys::RTCRay4;
pub type Hit4 = sys::RTCHit4;
pub type RayHit4 = sys::RTCRayHit4;

impl Ray4 {
    pub fn empty() -> Ray4 {
        Ray4::segment(
            [[0.0, 0.0, 0.0]; 4],
            [[0.0, 0.0, 0.0]; 4],
            [0.0; 4],
            [f32::INFINITY; 4],
        )
    }
    pub fn new(origin: [[f32; 3]; 4], dir: [[f32; 3]; 4]) -> Ray4 {
        Ray4::segment(origin, dir, [0.0; 4], [f32::INFINITY; 4])
    }
    pub fn segment(
        origin: [[f32; 3]; 4],
        dir: [[f32; 3]; 4],
        tnear: [f32; 4],
        tfar: [f32; 4],
    ) -> Ray4 {
        sys::RTCRay4 {
            org_x: [origin[0][0], origin[1][0], origin[2][0], origin[3][0]],
            org_y: [origin[0][1], origin[1][1], origin[2][1], origin[3][1]],
            org_z: [origin[0][2], origin[1][2], origin[2][2], origin[3][2]],
            dir_x: [dir[0][0], dir[1][0], dir[2][0], dir[3][0]],
            dir_y: [dir[0][1], dir[1][1], dir[2][1], dir[3][1]],
            dir_z: [dir[0][2], dir[1][2], dir[2][2], dir[3][2]],
            tnear,
            tfar,
            time: [0.0; 4],
            mask: [u32::MAX; 4],
            id: [0; 4],
            flags: [0; 4],
        }
    }
    pub fn iter(&self) -> SoARayIter<Ray4> { SoARayIter::new(self, 4) }
    pub fn iter_mut(&mut self) -> SoARayIterMut<Ray4> { SoARayIterMut::new(self, 4) }
}

impl SoARay for Ray4 {
    fn org(&self, i: usize) -> [f32; 3] { [self.org_x[i], self.org_y[i], self.org_z[i]] }
    fn set_org(&mut self, i: usize, o: [f32; 3]) {
        self.org_x[i] = o[0];
        self.org_y[i] = o[1];
        self.org_z[i] = o[2];
    }

    fn dir(&self, i: usize) -> [f32; 3] { [self.dir_x[i], self.dir_y[i], self.dir_z[i]] }
    fn set_dir(&mut self, i: usize, d: [f32; 3]) {
        self.dir_x[i] = d[0];
        self.dir_y[i] = d[1];
        self.dir_z[i] = d[2];
    }

    fn tnear(&self, i: usize) -> f32 { self.tnear[i] }
    fn set_tnear(&mut self, i: usize, near: f32) { self.tnear[i] = near; }

    fn tfar(&self, i: usize) -> f32 { self.tfar[i] }
    fn set_tfar(&mut self, i: usize, far: f32) { self.tfar[i] = far; }

    fn time(&self, i: usize) -> f32 { self.time[i] }
    fn set_time(&mut self, i: usize, time: f32) { self.time[i] = time; }

    fn mask(&self, i: usize) -> u32 { self.mask[i] }
    fn set_mask(&mut self, i: usize, mask: u32) { self.mask[i] = mask; }

    fn id(&self, i: usize) -> u32 { self.id[i] }
    fn set_id(&mut self, i: usize, id: u32) { self.id[i] = id; }

    fn flags(&self, i: usize) -> u32 { self.flags[i] }
    fn set_flags(&mut self, i: usize, flags: u32) { self.flags[i] = flags; }
}

impl Hit4 {
    pub fn new() -> Hit4 {
        sys::RTCHit4 {
            Ng_x: [0.0; 4],
            Ng_y: [0.0; 4],
            Ng_z: [0.0; 4],
            u: [0.0; 4],
            v: [0.0; 4],
            primID: [u32::MAX; 4],
            geomID: [u32::MAX; 4],
            instID: [[u32::MAX; 4]],
        }
    }
    pub fn any_hit(&self) -> bool { self.hits().any(|h| h) }
    pub fn hits<'a>(&'a self) -> impl Iterator<Item = bool> + 'a {
        self.geomID.iter().map(|g| *g != u32::MAX)
    }
    pub fn iter(&self) -> SoAHitIter<Hit4> { SoAHitIter::new(self, 4) }
    pub fn iter_hits<'a>(&'a self) -> impl Iterator<Item = SoAHitRef<Hit4>> + 'a {
        SoAHitIter::new(self, 4).filter(|h| h.hit())
    }
}

impl SoAHit for Hit4 {
    fn normal(&self, i: usize) -> [f32; 3] { [self.Ng_x[i], self.Ng_y[i], self.Ng_z[i]] }
    fn set_normal(&mut self, i: usize, n: [f32; 3]) {
        self.Ng_x[i] = n[0];
        self.Ng_y[i] = n[1];
        self.Ng_z[i] = n[2];
    }

    fn uv(&self, i: usize) -> (f32, f32) { (self.u[i], self.v[i]) }
    fn set_u(&mut self, i: usize, u: f32) { self.u[i] = u; }
    fn set_v(&mut self, i: usize, v: f32) { self.v[i] = v; }

    fn prim_id(&self, i: usize) -> u32 { self.primID[i] }
    fn set_prim_id(&mut self, i: usize, id: u32) { self.primID[i] = id; }

    fn geom_id(&self, i: usize) -> u32 { self.geomID[i] }
    fn set_geom_id(&mut self, i: usize, id: u32) { self.geomID[i] = id; }

    fn inst_id(&self, i: usize) -> u32 { self.instID[0][i] }
    fn set_inst_id(&mut self, i: usize, id: u32) { self.instID[0][i] = id; }
}

impl RayHit4 {
    pub fn new(ray: Ray4) -> RayHit4 {
        sys::RTCRayHit4 {
            ray,
            hit: Hit4::new(),
        }
    }
    pub fn iter(&self) -> std::iter::Zip<SoARayIter<Ray4>, SoAHitIter<Hit4>> {
        self.ray.iter().zip(self.hit.iter())
    }
}

pub struct RayPacket<const N: usize> {
    pub org_x: [f32; N],
    pub org_y: [f32; N],
    pub org_z: [f32; N],
    pub tnear: [f32; N],
    pub dir_x: [f32; N],
    pub dir_y: [f32; N],
    pub dir_z: [f32; N],
    pub time: [f32; N],
    pub tfar: [f32; N],
    pub mask: [u32; N],
    pub id: [u32; N],
    pub flags: [u32; N],
}

pub struct HitPacket<const N: usize> {
    pub Ng_x: [f32; N],
    pub Ng_y: [f32; N],
    pub Ng_z: [f32; N],
    pub u: [f32; N],
    pub v: [f32; N],
    pub primID: [u32; N],
    pub geomID: [u32; N],
    pub instID: [[u32; 1]; N],
}

pub struct RayHitPacket<const N: usize> {
    pub ray: RayPacket<N>,
    pub hit: HitPacket<N>,
}
