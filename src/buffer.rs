use crate::Error;
use std::marker::PhantomData;
use std::ops::{Bound, Index, IndexMut, RangeBounds};
use std::{mem, ptr};

use crate::device::Device;
use crate::sys::*;
use crate::BufferType;

/// Handle to a buffer managed by Embree.
pub struct Buffer {
    pub(crate) device: Device,
    pub(crate) handle: RTCBuffer,
    pub(crate) size: usize,
}

impl Buffer {
    /// Creates a new data buffer of the given size.
    pub(crate) fn new(device: Device, size: usize) -> Result<Buffer, Error> {
        // Pad to a multiple of 16 bytes
        let size = if size % 16 == 0 {
            size
        } else {
            (size + 15) & !15
        };
        let handle = unsafe { rtcNewBuffer(device.handle, size) };
        if handle.is_null() {
            Err(device.get_error())
        } else {
            Ok(Buffer {
                device,
                handle,
                size,
            })
        }
    }

    /// Slices into the buffer for the given range.
    ///
    /// Choosing a range with no end will slice to the end of the buffer:
    ///
    /// ```
    /// buffer.slice(16..)
    /// ```
    ///
    /// Choosing a totally unbounded range will use the entire buffer:
    ///
    /// ```
    /// buffer.slice(..)
    /// ```
    pub fn slice<S: RangeBounds<usize>>(&self, bounds: S) -> BufferSlice<'_> {
        let (offset, size) = range_bounds_to_offset_and_size(bounds);
        BufferSlice {
            buffer: self,
            offset,
            size,
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseBuffer(self.handle);
        }
    }
}

static_assertions::assert_impl_all!(Buffer: Send, Sync);

/// Slice into a [`Buffer`].
///
/// Created with [`Buffer::slice`].
#[derive(Debug, Clone, Copy)]
pub struct BufferSlice<'a> {
    buffer: &'a Buffer,
    offset: usize,
    size: Option<usize>,
}

static_assertions::assert_impl_all!(BufferSlice: Send, Sync);

impl<'a> BufferSlice<'a> {
    /// Returns a immutable slice into the buffer.
    pub fn view(&self) -> BufferView<'a> {
        BufferView {
            buffer: self.buffer,
            offset: self.offset,
            size: self.size.unwrap_or(self.buffer.size - self.offset),
            _marker: PhantomData,
        }
    }

    /// Returns a mutable slice into the buffer.
    pub fn view_mut(&mut self) -> BufferViewMut<'a> {
        BufferViewMut {
            buffer: self.buffer,
            offset: self.offset,
            size: self.size.unwrap_or(self.buffer.size - self.offset),
            _marker: PhantomData,
        }
    }
}

/// A read-only view of a buffer.
pub struct BufferView<'a, T: 'a> {
    slice: BufferSlice<'a>,
    data: MappedBuffer<'a, T>,
}

/// A mutable view of a buffer.
pub struct BufferViewMut<'a, T: 'a> {
    slice: BufferSlice<'a>,
    data: MappedBuffer<'a, T>,
}

#[derive(Debug)]
pub struct MappedBuffer<'a, T: 'a> {
    ptr: *mut T,
    len: usize,
    _marker: PhantomData<&'a mut T>,
}

impl<T> AsRef<[T]> for BufferView<'_, T> {
    fn as_ref(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.data.ptr, self.len) }
    }
}

impl<T> AsMut<[T]> for BufferViewMut<'_, T> {
    fn as_mut(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.data.ptr, self.len) }
    }
}

fn range_bounds_to_offset_and_size<S: RangeBounds>(
    bounds: RangeBounds<usize>,
) -> (usize, Option<usize>) {
    let offset = match bounds.start_bound() {
        Bound::Included(&n) => n,
        Bound::Excluded(&n) => n + 1,
        Bound::Unbounded => 0,
    };
    let size = match bounds.end_bound() {
        Bound::Included(&n) => Some(n - offset + 1),
        Bound::Excluded(&n) => Some(n - offset),
        Bound::Unbounded => None,
    };

    (offset, size)
}
