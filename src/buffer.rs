use std::marker::PhantomData;
use std::mem;
use std::ops::{Index, IndexMut};

use sys::*;
use device::Device;

pub struct Buffer<'a, T> {
    device: &'a Device,
    pub (crate) handle: RTCBuffer,
    bytes: usize,
    marker: PhantomData<T>
}

impl<'a, T> Buffer<'a, T> {
    /// Allocate a buffer with some raw capacity in bytes
    pub fn raw(device: &'a Device, bytes: usize) -> Buffer<'a, T> {
        Buffer {
            device: device,
            handle: unsafe { rtcNewBuffer(device.handle, bytes) },
            bytes: bytes,
            marker: PhantomData
        }
    }
    pub fn new(device: &'a Device, len: usize) -> Buffer<'a, T> {
        let bytes = len * mem::size_of::<T>();
        Buffer {
            device: device,
            handle: unsafe { rtcNewBuffer(device.handle, bytes) },
            bytes: bytes,
            marker: PhantomData
        }
    }
    pub fn map<'b>(&'b mut self) -> MappedBuffer<'b, T> {
        let len = self.bytes / mem::size_of::<T>();
        let slice = unsafe { rtcGetBufferData(self.handle) as *mut T };
        MappedBuffer {
            marker: PhantomData,
            slice: slice,
            len: len,
        }
    }
}

impl<'a, T> Drop for Buffer<'a, T> {
    fn drop(&mut self) {
        unsafe { rtcReleaseBuffer(self.handle); }
    }
}

pub struct MappedBuffer<'a, T: 'a> {
    marker: PhantomData<&'a mut Buffer<'a, T>>,
    slice: *mut T,
    len: usize,
}

impl<'a, T: 'a> MappedBuffer<'a, T> {
    pub fn len(&self) -> usize {
        self.len
    }
}

impl<'a, T: 'a> Index<usize> for MappedBuffer<'a, T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        // TODO: We should only check in debug build
        if index >= self.len {
            panic!("MappedBuffer index out of bounds");
        }
        unsafe { &*self.slice.offset(index as isize) }
    }
}

impl<'a, T: 'a> IndexMut<usize> for MappedBuffer<'a, T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        if index >= self.len {
            panic!("MappedBuffer index out of bounds");
        }
        unsafe { &mut *self.slice.offset(index as isize) }
    }
}

