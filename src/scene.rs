use std::cell::RefCell;
use std::mem;

use sys::*;

pub struct Scene {
    handle: RefCell<RTCScene>,
}
// TODO: Use the bitflags crate to generate the bitflags
// mask the scene will take to construct itself

