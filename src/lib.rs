//! TODO: Docs

use std::{u32, f32};
use std::iter::Iterator;

#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
pub mod sys;
pub use sys::*;

impl sys::RTCRay {
    /// Create a new ray starting at `origin` and heading in direction `dir`
    pub fn new(origin: &[f32; 3], dir: &[f32; 3]) -> sys::RTCRay {
        sys::RTCRay {
            org: *origin,
            align0: 0.0,
            dir: *dir,
            align1: 0.0,
            tnear: 0.0,
            tfar: f32::INFINITY,
            time: 0.0,
            mask: u32::MAX,
            Ng: [0.0; 3],
            align2: 0.0,
            u: 0.0,
            v: 0.0,
            geomID: u32::MAX,
            primID: u32::MAX,
            instID: u32::MAX,
            __bindgen_padding_0: [0; 3],
        }
    }
}

/// This will need a bit more thought on how I really want to structure
/// the API design for working with these buffers and the objects
/// associated with them.
pub struct BufferMapping<'a, T: 'a> {
    slice: &'a mut [T],
    // TODO: it also needs the scene, geom id and buffer type to unmap
}
impl<'a, T: 'a> BufferMapping<'a, T> {
    pub fn iter<'b>(&'b self) -> BufferMappingIter<'b, T> {
        BufferMappingIter::new(self.slice)
    }
}
impl<'a, T: 'a> Drop for BufferMapping<'a, T> {
    fn drop(&mut self) {
        // TODO: unmap it
        //embree::rtcUnmapBuffer(scene, geom_id, embree::RTCBufferType::RTC_INDEX_BUFFER);
    }
}

/// TODO: These should be working on the raw pointers, like the slice iterators.
pub struct BufferMappingIter<'a, T: 'a> {
    slice: &'a [T],
    next: usize,
}
impl<'a, T: 'a> BufferMappingIter<'a, T> {
    fn new(slice: &'a [T]) -> BufferMappingIter<'a, T> {
        BufferMappingIter { slice: slice, next: 0 }
    }
}
impl<'a, T: 'a> Iterator for BufferMappingIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        let res = self.slice.get(self.next);
        self.next = self.next + 1;
        res
    }
}

