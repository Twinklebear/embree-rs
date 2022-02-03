use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use std::sync::Arc;
use std::{mem, ptr};

use crate::device::Device;
use crate::sys::*;
use crate::BufferType;

#[derive(Copy, Clone)]
struct BufferAttachment {
    geom: RTCGeometry,
    buf_type: BufferType,
    slot: u32,
}

impl BufferAttachment {
    fn none() -> BufferAttachment {
        BufferAttachment {
            geom: ptr::null_mut(),
            buf_type: BufferType::VERTEX,
            slot: std::u32::MAX,
        }
    }
    fn is_attached(&self) -> bool {
        self.geom != ptr::null_mut()
    }
}

// TODO: To handle this nicely for sharing/re-using/changing buffer views
// we basically need an API/struct for making buffer views of existing
// larger buffers.
pub struct Buffer<T> {
    device: Arc<Device>,
    pub(crate) handle: RTCBuffer,
    // TODO: We need a list of RTCGeometry handles
    // that we're attached to to mark buffers as updated on
    // the geometries.
    bytes: usize,
    attachment: BufferAttachment,
    marker: PhantomData<T>,
}

impl<T> Buffer<T> {
    /// Allocate a buffer with some raw capacity in bytes
    pub fn raw(device: Arc<Device>, bytes: usize) -> Buffer<T> {
        // Pad to a multiple of 16 bytes
        let bytes = if bytes % 16 == 0 {
            bytes
        } else {
            bytes + bytes / 16
        };
        let h = unsafe { rtcNewBuffer(device.handle, bytes) };
        Buffer {
            device: device,
            handle: h,
            bytes: bytes,
            attachment: BufferAttachment::none(),
            marker: PhantomData,
        }
    }
    pub fn new(device: Arc<Device>, len: usize) -> Buffer<T> {
        let mut bytes = len * mem::size_of::<T>();
        // Pad to a multiple of 16 bytes
        bytes = if bytes % 16 == 0 {
            bytes
        } else {
            bytes + bytes / 16
        };
        let h = unsafe { rtcNewBuffer(device.handle, bytes) };
        Buffer {
            device: device,
            handle: h,
            bytes: bytes,
            attachment: BufferAttachment::none(),
            marker: PhantomData,
        }
    }
    pub fn map<'a>(&'a mut self) -> MappedBuffer<'a, T> {
        let len = self.bytes / mem::size_of::<T>();
        let slice = unsafe { rtcGetBufferData(self.handle) as *mut T };
        MappedBuffer {
            buffer: PhantomData,
            attachment: self.attachment,
            slice: slice,
            len: len,
        }
    }
    pub(crate) fn set_attachment(&mut self, geom: RTCGeometry, buf_type: BufferType, slot: u32) {
        self.attachment.geom = geom;
        self.attachment.buf_type = buf_type;
        self.attachment.slot = slot;
    }
}

impl<T> Drop for Buffer<T> {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseBuffer(self.handle);
        }
    }
}

pub struct MappedBuffer<'a, T: 'a> {
    buffer: PhantomData<&'a mut Buffer<T>>,
    attachment: BufferAttachment,
    slice: *mut T,
    len: usize,
}

impl<'a, T: 'a> MappedBuffer<'a, T> {
    pub fn len(&self) -> usize {
        self.len
    }
}

impl<'a, T: 'a> Drop for MappedBuffer<'a, T> {
    fn drop(&mut self) {
        if self.attachment.is_attached() {
            // TODO: support for attaching one buffer to multiple geoms?
            unsafe {
                rtcUpdateGeometryBuffer(
                    self.attachment.geom,
                    self.attachment.buf_type,
                    self.attachment.slot,
                );
            }
        }
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
