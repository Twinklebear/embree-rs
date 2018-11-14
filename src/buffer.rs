use std::marker::PhantomData;
use std::mem;
use std::ops::{Index, IndexMut};

use device::Device;
use sys::*;

pub struct Buffer<'a, T> {
    device: &'a Device,
    pub(crate) handle: RTCBuffer,
    // TODO: We need a list of RTCGeometry handles
    // that we're attached to to mark buffers as updated on
    // the geometries.
    bytes: usize,
    marker: PhantomData<T>,
}

// TODO: To handle this nicely for sharing/re-using/changing buffer views
// we basically need an API/struct for making buffer views of existing
// larger buffers.

impl<'a, T> Buffer<'a, T> {
    /// Allocate a buffer with some raw capacity in bytes
    pub fn raw(device: &'a Device, bytes: usize) -> Buffer<'a, T> {
        // Pad to a multiple of 16 bytes
        let bytes = if bytes % 16 == 0 { bytes } else { bytes + bytes / 16};
        Buffer {
            device: device,
            handle: unsafe { rtcNewBuffer(device.handle, bytes) },
            bytes: bytes,
            marker: PhantomData,
        }
    }
    pub fn new(device: &'a Device, len: usize) -> Buffer<'a, T> {
        let mut bytes = len * mem::size_of::<T>();
        // Pad to a multiple of 16 bytes
        bytes = if bytes % 16 == 0 { bytes } else { bytes + bytes / 16};
        Buffer {
            device: device,
            handle: unsafe { rtcNewBuffer(device.handle, bytes) },
            bytes: bytes,
            marker: PhantomData,
        }
    }
    pub fn map<'b>(&'b mut self) -> MappedBuffer<'a, 'b, T> {
        let len = self.bytes / mem::size_of::<T>();
        let slice = unsafe { rtcGetBufferData(self.handle) as *mut T };
        MappedBuffer {
            buffer: self,
            slice: slice,
            len: len,
        }
    }
}

impl<'a, T> Drop for Buffer<'a, T> {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseBuffer(self.handle);
        }
    }
}

pub struct MappedBuffer<'a, 'b, T: 'a> {
    buffer: &'b mut Buffer<'a, T>,
    slice: *mut T,
    len: usize,
}

impl<'a, 'b, T: 'a> MappedBuffer<'a, 'b, T> {
    pub fn len(&self) -> usize {
        self.len
    }
}

impl<'a, 'b, T> Drop for MappedBuffer<'a, 'b, T> {
    fn drop(&mut self) {
        unsafe {
            // TODO: We should call the rtcSetGeometryBufferUpdated function
            // but we need to know the geometry we're attached to now.
            // Can we be attached to multiple?
        }
    }
}

impl<'a, 'b, T: 'a> Index<usize> for MappedBuffer<'a, 'b, T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        // TODO: We should only check in debug build
        if index >= self.len {
            panic!("MappedBuffer index out of bounds");
        }
        unsafe { &*self.slice.offset(index as isize) }
    }
}

impl<'a, 'b, T: 'a> IndexMut<usize> for MappedBuffer<'a, 'b, T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        if index >= self.len {
            panic!("MappedBuffer index out of bounds");
        }
        unsafe { &mut *self.slice.offset(index as isize) }
    }
}
