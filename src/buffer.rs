use crate::Error;
use std::marker::PhantomData;
use std::mem;
use std::num::NonZeroUsize;
use std::ops::{Bound, RangeBounds};

use crate::device::Device;
use crate::sys::*;

/// Non-zero integer type used to describe the size of a buffer.
pub type BufferSize = NonZeroUsize;

/// Handle to a buffer managed by Embree.
pub struct Buffer {
    pub(crate) device: Device,
    pub(crate) handle: RTCBuffer,
    pub(crate) size: BufferSize,
}

impl Clone for Buffer {
    fn clone(&self) -> Self {
        unsafe { rtcRetainBuffer(self.handle) };
        Buffer {
            device: self.device.clone(),
            handle: self.handle,
            size: self.size,
        }
    }
}

impl Buffer {
    /// Creates a new data buffer of the given size.
    pub(crate) fn new(device: Device, size: BufferSize) -> Result<Buffer, Error> {
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

    /// Returns the a slice of the buffer.
    pub fn slice<S: RangeBounds<usize>>(&self, bounds: S) -> BufferSlice {
        let start = match bounds.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };
        let end = match bounds.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => self.size.get(),
        };
        BufferSlice {
            buffer: self.clone(),
            offset: start,
            size: NonZeroUsize::new(end - start).unwrap(),
        }
    }

    /// Slices into the buffer for the given range.
    ///
    /// # Arguments
    ///
    /// * `range` - The range of indices to slice into the buffer.
    ///   - Ranges with no end will slice to the end of the buffer.
    ///   - Totally unbounded range (..) will slice the entire buffer.
    pub fn mapped_range<S: RangeBounds<usize>, T>(&self, bounds: S) -> BufferView<'_, T> {
        let (offset, size) = range_bounds_to_offset_and_size(bounds);
        let size = size.unwrap_or_else(|| self.size.get() - offset);
        debug_assert!(offset + size <= self.size.get() && offset < self.size.get());
        let range = BufferSlice {
            buffer: self,
            offset,
            size: BufferSize::new(size).unwrap(),
        };
        BufferView::new(self, range)
    }

    /// Mutable slice into the buffer for the given range.
    pub fn mapped_range_mut<S: RangeBounds<usize>>(&mut self, bounds: S) -> BufferViewMut<'_> {
        let (offset, size) = range_bounds_to_offset_and_size(bounds);
        let size = size.unwrap_or_else(|| self.size.get() - offset);
        debug_assert!(offset + size <= self.size.get() && offset < self.size.get());
        let range = BufferSlice {
            buffer: self,
            offset,
            size: BufferSize::new(size).unwrap(),
        };
        BufferViewMut::new(self, range)
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

/// A read-only view into mapped buffer.
#[derive(Debug)]
pub struct BufferView<'a, T: 'a> {
    range: BufferSlice,
    slice: BufferMappedRange<'a, T>,
    marker: PhantomData<&'a Buffer>,
}

/// A write-only view into mapped buffer.
#[derive(Debug)]
pub struct BufferViewMut<'a, T: 'a> {
    range: BufferSlice,
    slice: BufferMappedRange<'a, T>,
    marker: PhantomData<&'a mut Buffer>,
}

/// Slice into a [`Buffer`].
///
/// Created with [`Buffer::slice`].
#[derive(Debug)]
pub struct BufferSlice {
    /// The buffer this slice is a part of.
    pub(crate) buffer: Buffer,
    /// The offset into the buffer in bytes.
    pub(crate) offset: usize,
    /// The size of the slice in bytes.
    pub(crate) size: BufferSize,
}

static_assertions::assert_impl_all!(BufferSlice: Send, Sync);

impl<'a> BufferSlice<'a> {
    fn view<T>(&self) -> BufferView<'a, T> {
        let slice = BufferMappedRange::new(&self.buffer, self.offset, self.size)?;
        BufferView {
            range: *self,
            slice,
            marker: PhantomData,
        }
    }

    fn view_mut<T>(&self) -> BufferViewMut<'a, T> {
        let slice = BufferMappedRange::new(&self.buffer, self.offset, self.size)?;
        BufferViewMut {
            range: *self,
            slice,
            marker: PhantomData,
        }
    }
}

#[derive(Debug)]
struct BufferMappedRange<'a, T: 'a> {
    ptr: *mut T,
    len: usize,
}

impl<'a, T: 'a> BufferMappedRange<'a, T> {
    /// Creates a new slice from the given Buffer with the given offset and size.
    ///
    /// The offset and size must be in bytes and must be a multiple of the size of `T`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the given offset and size are valid.
    fn new(
        buffer: &'a Buffer,
        offset: usize,
        size: usize,
    ) -> Result<BufferMappedRange<'a, T>, Error> {
        debug_assert!(
            size % mem::size_of::<T>() == 0,
            "Size of the range of the mapped buffer must be multiple of T!"
        );
        debug_assert!(
            offset % mem::size_of::<T>() == 0,
            "Offset must be multiple of T!"
        );
        let ptr = unsafe {
            let ptr = rtcGetBufferData(buffer.handle) as *const u8;
            if ptr.is_null() {
                return Err(buffer.device.get_error());
            }
            ptr.offset(offset as isize)
        } as *mut T;
        Ok(BufferMappedRange {
            ptr,
            len: size / mem::size_of::<T>(),
        })
    }

    fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl<T> AsRef<[T]> for BufferView<'_, T> {
    fn as_ref(&self) -> &[T] {
        self.slice.as_slice()
    }
}

impl<T> AsMut<[T]> for BufferViewMut<'_, T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.slice.as_mut_slice()
    }
}

/// Converts a range bounds into an offset and size.
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
