use crate::{sys::*, BuildFlags, BuildPrimitive, BuildQuality, Device, Error};

pub struct Bvh {
    handle: RTCBVH,
}

impl Clone for Bvh {
    fn clone(&self) -> Self {
        unsafe { rtcRetainBVH(self.handle) }
        Self {
            handle: self.handle,
        }
    }
}

impl Drop for Bvh {
    fn drop(&mut self) { unsafe { rtcReleaseBVH(self.handle) } }
}

impl Bvh {
    pub(crate) fn new(device: &Device) -> Result<Self, Error> {
        let handle = unsafe { rtcNewBVH(device.handle) };
        if handle.is_null() {
            Err(device.get_error())
        } else {
            Ok(Self { handle })
        }
    }
}

pub struct BvhBuilderUserData {
    create_node_fn: *mut std::os::raw::c_void,
    set_node_children_fn: *mut std::os::raw::c_void,
    set_node_bounds_fn: *mut std::os::raw::c_void,
    create_leaf_fn: *mut std::os::raw::c_void,
    split_primitive_fn: *mut std::os::raw::c_void,
    progress_monitor_function: *mut std::os::raw::c_void,
}

pub struct BvhBuilder<'a> {
    bvh: &'a Bvh,
    quality: Option<BuildQuality>,
    flags: Option<BuildFlags>,
    max_branching_factor: Option<u32>,
    max_depth: Option<u32>,
    sah_block_size: Option<u32>,
    min_leaf_size: Option<u32>,
    max_leaf_size: Option<u32>,
    traversal_cost: Option<u32>,
    intersection_cost: Option<u32>,
    primitives: Option<Vec<BuildPrimitive>>,
    ready: u32,
}

impl<'a> BvhBuilder<'a> {
    pub fn new(bvh: &'a Bvh) -> Self {
        Self {
            bvh,
            quality: None,
            flags: None,
            max_branching_factor: None,
            max_depth: None,
            sah_block_size: None,
            min_leaf_size: None,
            max_leaf_size: None,
            traversal_cost: None,
            intersection_cost: None,
            primitives: None,
            ready: 0,
        }
    }

    pub fn quality(mut self, quality: BuildQuality) -> Self {
        self.quality = Some(quality);
        self.ready |= 1;
        self
    }

    pub fn flags(mut self, flags: BuildFlags) -> Self {
        self.flags = Some(flags);
        self.ready |= 1 << 1;
        self
    }

    pub fn max_branching_factor(mut self, max_branching_factor: u32) -> Self {
        self.max_branching_factor = Some(max_branching_factor);
        self.ready |= 1 << 2;
        self
    }

    pub fn max_depth(mut self, max_depth: u32) -> Self {
        self.max_depth = Some(max_depth);
        self.ready |= 1 << 3;
        self
    }

    pub fn sah_block_size(mut self, sah_block_size: u32) -> Self {
        self.sah_block_size = Some(sah_block_size);
        self.ready |= 1 << 4;
        self
    }

    pub fn min_leaf_size(mut self, min_leaf_size: u32) -> Self {
        self.min_leaf_size = Some(min_leaf_size);
        self.ready |= 1 << 5;
        self
    }

    pub fn max_leaf_size(mut self, max_leaf_size: u32) -> Self {
        self.max_leaf_size = Some(max_leaf_size);
        self.ready |= 1 << 6;
        self
    }

    pub fn traversal_cost(mut self, traversal_cost: u32) -> Self {
        self.traversal_cost = Some(traversal_cost);
        self.ready |= 1 << 7;
        self
    }

    pub fn intersection_cost(mut self, intersection_cost: u32) -> Self {
        self.intersection_cost = Some(intersection_cost);
        self.ready |= 1 << 8;
        self
    }

    pub fn primitives(mut self, primitives: Vec<BuildPrimitive>) -> Self {
        self.primitives = Some(primitives);
        self.ready |= 1 << 9;
        self
    }

    // TODO: build
}
