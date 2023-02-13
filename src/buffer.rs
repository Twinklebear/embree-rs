use crate::Error;
use std::{
    marker::PhantomData,
    mem,
    num::NonZeroUsize,
    ops::{Bound, Deref, DerefMut, RangeBounds},
};

use crate::{device::Device, sys::*};

/// Non-zero integer type used to describe the size of a buffer.
pub type BufferSize = NonZeroUsize;

/// Handle to a buffer managed by Embree.
#[derive(Debug)]
pub struct Buffer {
    pub(crate) device: Device,
    pub(crate) handle: RTCBuffer,
    pub(crate) size: BufferSize,
}

impl Clone for Buffer {
    fn clone(&self) -> Self {
        unsafe { rtcRetainBuffer(self.handle) };
        Buffer {
            handle: self.handle,
            size: self.size,
            device: self.device.clone(),
        }
    }
}

impl Buffer {
    /// Creates a new data buffer of the given size.
    pub(crate) fn new(device: &Device, size: BufferSize) -> Result<Buffer, Error> {
        // Pad to a multiple of 16 bytes
        let size = if size.get() % 16 == 0 {
            size.get()
        } else {
            (size.get() + 15) & !15
        };
        let handle = unsafe { rtcNewBuffer(device.handle, size) };
        if handle.is_null() {
            Err(device.get_error())
        } else {
            Ok(Buffer {
                handle,
                size: NonZeroUsize::new(size).unwrap(),
                device: device.clone(),
            })
        }
    }

    pub fn handle(&self) -> RTCBuffer { self.handle }

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
        BufferSlice::Created {
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
    pub fn mapped_range<'a, S: RangeBounds<usize>, T>(&'a self, bounds: S) -> BufferView<'a, T> {
        let (offset, size) = range_bounds_to_offset_and_size(bounds);
        let size = size.unwrap_or_else(|| self.size.get() - offset);
        debug_assert!(offset + size <= self.size.get() && offset < self.size.get());
        BufferView::new(self, offset, BufferSize::new(size).unwrap()).unwrap()
    }

    /// Mutable slice into the buffer for the given range.
    pub fn mapped_range_mut<S: RangeBounds<usize>, T>(
        &mut self,
        bounds: S,
    ) -> BufferViewMut<'_, T> {
        let (offset, size) = range_bounds_to_offset_and_size(bounds);
        let size = size.unwrap_or_else(|| self.size.get() - offset);
        debug_assert!(offset + size <= self.size.get() && offset < self.size.get());
        BufferViewMut::new(self, offset, BufferSize::new(size).unwrap()).unwrap()
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseBuffer(self.handle);
        }
    }
}

/// A read-only view into mapped buffer.
#[derive(Debug)]
pub struct BufferView<'a, T: 'a> {
    range: BufferSlice,
    mapped: BufferMappedRange<'a, T>,
    marker: PhantomData<&'a T>,
}

/// A write-only view into mapped buffer.
#[derive(Debug)]
pub struct BufferViewMut<'a, T: 'a> {
    range: BufferSlice,
    mapped: BufferMappedRange<'a, T>,
    marker: PhantomData<&'a mut T>,
}

/// Slice into a [`Buffer`].
///
/// Created with [`Buffer::slice`].
#[derive(Debug, Clone)]
pub enum BufferSlice {
    /// Slice created from a [`Buffer`].
    Created {
        /// The buffer this slice is a part of.
        buffer: Buffer,
        /// The offset into the buffer in bytes.
        offset: usize,
        /// The size of the slice in bytes.
        size: BufferSize,
    },
    /// Slice managed by Embree internally.
    Managed {
        ptr: *mut ::std::os::raw::c_void,
        size: BufferSize,
        marker: PhantomData<*mut ::std::os::raw::c_void>,
    },
}

impl BufferSlice {
    pub fn view<T>(&self) -> Result<BufferView<'_, T>, Error> {
        match self {
            BufferSlice::Created {
                buffer,
                offset,
                size,
            } => {
                let slice = BufferMappedRange::from_buffer(buffer, *offset, size.get())?;
                Ok(BufferView {
                    range: self.clone(),
                    mapped: slice,
                    marker: PhantomData,
                })
            }
            BufferSlice::Managed { ptr, size, .. } => {
                debug_assert!(
                    size.get() % mem::size_of::<T>() == 0,
                    "Size of the range of the mapped buffer must be multiple of T!"
                );
                let len = size.get() / mem::size_of::<T>();
                let slice = unsafe { BufferMappedRange::from_raw_parts(*ptr as *mut T, len) };
                Ok(BufferView {
                    range: self.clone(),
                    mapped: slice,
                    marker: PhantomData,
                })
            }
        }
    }

    pub fn view_mut<T>(&self) -> Result<BufferViewMut<'_, T>, Error> {
        match self {
            BufferSlice::Created {
                buffer,
                offset,
                size,
            } => Ok(BufferViewMut {
                range: self.clone(),
                mapped: BufferMappedRange::from_buffer(buffer, *offset, size.get())?,
                marker: PhantomData,
            }),
            BufferSlice::Managed { ptr, size, .. } => {
                debug_assert!(
                    size.get() % mem::size_of::<T>() == 0,
                    "Size of the range of the mapped buffer must be multiple of T!"
                );
                let len = size.get() / mem::size_of::<T>();
                let slice = unsafe { BufferMappedRange::from_raw_parts(*ptr as *mut T, len) };
                Ok(BufferViewMut {
                    range: self.clone(),
                    mapped: slice,
                    marker: PhantomData,
                })
            }
        }
    }
}

impl<'a, T> BufferView<'a, T> {
    /// Creates a new slice from the given Buffer with the given offset and
    /// size. Only used internally by [`Buffer::mapped_range`].
    fn new(
        buffer: &'a Buffer,
        offset: usize,
        size: BufferSize,
    ) -> Result<BufferView<'a, T>, Error> {
        Ok(BufferView {
            range: BufferSlice::Created {
                buffer: buffer.clone(),
                offset,
                size,
            },
            mapped: BufferMappedRange::from_buffer(buffer, offset, size.into())?,
            marker: PhantomData,
        })
    }
}

impl<'a, T> BufferViewMut<'a, T> {
    /// Creates a new slice from the given Buffer with the given offset and
    /// size. Only used internally by [`Buffer::mapped_range_mut`].
    fn new(
        buffer: &'a Buffer,
        offset: usize,
        size: BufferSize,
    ) -> Result<BufferViewMut<'a, T>, Error> {
        Ok(BufferViewMut {
            range: BufferSlice::Created {
                buffer: buffer.clone(),
                offset,
                size,
            },
            mapped: BufferMappedRange::from_buffer(buffer, offset, size.into())?,
            marker: PhantomData,
        })
    }
}

#[derive(Debug)]
struct BufferMappedRange<'a, T: 'a> {
    ptr: *mut T,
    len: usize,
    marker: PhantomData<&'a mut T>, // covariant without drop check
}

impl<'a, T: 'a> BufferMappedRange<'a, T> {
    /// Creates a new slice from the given Buffer with the given offset and
    /// size.
    ///
    /// The offset and size must be in bytes and must be a multiple of the size
    /// of `T`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the given offset and size are valid.
    fn from_buffer(
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
            marker: PhantomData,
        })
    }

    /// Creates a new slice from the given raw pointer and length.
    unsafe fn from_raw_parts(ptr: *mut T, len: usize) -> BufferMappedRange<'a, T> {
        BufferMappedRange {
            ptr,
            len,
            marker: PhantomData,
        }
    }

    fn as_slice(&self) -> &[T] { unsafe { std::slice::from_raw_parts(self.ptr, self.len) } }

    fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl<T> AsRef<[T]> for BufferView<'_, T> {
    fn as_ref(&self) -> &[T] { self.mapped.as_slice() }
}

impl<T> AsMut<[T]> for BufferViewMut<'_, T> {
    fn as_mut(&mut self) -> &mut [T] { self.mapped.as_mut_slice() }
}

impl<T> Deref for BufferView<'_, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target { self.mapped.as_slice() }
}

impl<T> Deref for BufferViewMut<'_, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target { self.mapped.as_slice() }
}

impl<T> DerefMut for BufferViewMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target { self.mapped.as_mut_slice() }
}

/// Converts a range bounds into an offset and size.
fn range_bounds_to_offset_and_size<S: RangeBounds<usize>>(bounds: S) -> (usize, Option<usize>) {
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
