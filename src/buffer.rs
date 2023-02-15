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
            device: self.device.clone(),
            handle: self.handle,
            size: self.size,
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

    /// Returns the raw handle to the buffer.
    ///
    /// # Safety
    ///
    /// The handle returned by this function is a raw pointer to the
    /// underlying Embree buffer. It is not safe to use this handle
    /// outside of the Embree API.
    pub unsafe fn handle(&self) -> RTCBuffer { self.handle }

    /// Returns a slice of the buffer.
    ///
    /// This function only returns a slice of the buffer, and does not
    /// map the buffer into memory. To map the buffer into memory, use
    /// [`Buffer::mapped_range`] or [`BufferSlice::view`] to create a
    /// read-only view of the buffer, or [`Buffer::mapped_range_mut`] or
    /// [`BufferSlice::view_mut`] to create a mutable view of the buffer.
    ///
    /// # Arguments
    ///
    /// * `bounds` - The range of bytes to slice into the buffer.
    pub fn slice<S: RangeBounds<usize>>(&self, bounds: S) -> BufferSlice {
        let (offset, size) = range_bounds_to_offset_and_size(bounds);
        let size = size.unwrap_or_else(|| self.size.get() - offset);
        debug_assert!(offset + size <= self.size.get() && offset < self.size.get());
        BufferSlice::Buffer {
            buffer: self,
            offset,
            size: NonZeroUsize::new(size).unwrap(),
        }
    }

    /// Slices into the buffer for the given range.
    ///
    /// # Arguments
    ///
    /// * `bounds` - The range of indices to slice into the buffer.
    ///   - Ranges with no end will slice to the end of the buffer.
    ///   - Totally unbounded range (..) will slice the entire buffer.
    pub fn mapped_range<S: RangeBounds<usize>, T>(&self, bounds: S) -> BufferView<'_, T> {
        let (offset, size) = range_bounds_to_offset_and_size(bounds);
        let size = size.unwrap_or_else(|| self.size.get() - offset);
        debug_assert!(offset + size <= self.size.get() && offset < self.size.get());
        BufferView::new(self, offset, BufferSize::new(size).unwrap()).unwrap()
    }

    /// Mutable slice into the buffer for the given range.
    ///
    /// # Arguments
    ///
    /// * `bounds` - The range of indices to slice into the buffer.
    ///  - Ranges with no end will slice to the end of the buffer.
    /// - Totally unbounded range (..) will slice the entire buffer.
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
pub struct BufferView<'buf, T: 'buf> {
    mapped: BufferMappedRange<'buf, T>,
    marker: PhantomData<&'buf T>,
}

/// A write-only view into mapped buffer.
#[derive(Debug)]
pub struct BufferViewMut<'buf, T: 'buf> {
    mapped: BufferMappedRange<'buf, T>,
    marker: PhantomData<&'buf mut T>,
}

/// Slice into a region of memory. This can either be a slice to a [`Buffer`] or
/// a slice to memory managed by Embree (mostly created from
/// [`rtcSetNewGeometryBuffer`]) or from user owned memory.
#[derive(Debug, Clone, Copy)]
pub enum BufferSlice<'src> {
    /// Slice into a [`Buffer`] object.
    Buffer {
        buffer: &'src Buffer,
        offset: usize,
        size: BufferSize,
    },
    /// Slice into memory managed by Embree.
    Internal {
        ptr: *mut ::std::os::raw::c_void,
        size: BufferSize,
        marker: PhantomData<&'src mut [::std::os::raw::c_void]>,
    },
    /// Slice into user borrowed/owned memory.
    User {
        ptr: *const u8,
        offset: usize,
        size: BufferSize,
        marker: PhantomData<&'src mut [u8]>,
    },
}

impl<'buf, T> From<&'buf [T]> for BufferSlice<'buf> {
    fn from(vec: &'buf [T]) -> Self { BufferSlice::from_slice(vec, ..) }
}

impl<'src> BufferSlice<'src> {
    /// Creates a new [`BufferSlice`] from a user owned buffer.
    ///
    /// # Arguments
    ///
    /// * `buffer` - The buffer to create a slice from.
    /// * `bounds` - The range of indices to slice into the buffer. Different
    ///   from [`Buffer::slice`],
    pub fn from_slice<'slice, T, S: RangeBounds<usize>>(slice: &'slice [T], bounds: S) -> Self {
        let (first, count) = range_bounds_to_offset_and_size(bounds);
        let count = count.unwrap_or_else(|| slice.len() - first);
        debug_assert!(
            first + count <= slice.len() && first < slice.len(),
            "Invalid slice range"
        );
        let elem_size = mem::size_of::<T>();
        BufferSlice::User {
            ptr: slice.as_ptr() as *const u8,
            offset: first * elem_size,
            size: BufferSize::new((first + count) * elem_size).unwrap(),
            marker: PhantomData,
        }
    }

    pub fn view<T>(&self) -> Result<BufferView<'src, T>, Error> {
        match self {
            BufferSlice::Buffer {
                buffer,
                offset,
                size,
            } => {
                let mapped = BufferMappedRange::from_buffer(buffer, *offset, size.get())?;
                Ok(BufferView {
                    // slice: *self,
                    mapped,
                    marker: PhantomData,
                })
            }
            BufferSlice::Internal { ptr, size, .. } => {
                debug_assert!(
                    size.get() % mem::size_of::<T>() == 0,
                    "Size of the range of the mapped buffer must be multiple of T!"
                );
                let len = size.get() / mem::size_of::<T>();
                let mapped = unsafe { BufferMappedRange::from_raw_parts(*ptr as *mut T, len) };
                Ok(BufferView {
                    //slice: *self,
                    mapped,
                    marker: PhantomData,
                })
            }
            BufferSlice::User {
                ptr, offset, size, ..
            } => {
                // TODO(yang): should we allow this?
                debug_assert!(
                    size.get() % mem::size_of::<T>() == 0,
                    "Size of the range of the mapped buffer must be multiple of T!"
                );
                let len = size.get() / mem::size_of::<T>();
                let mapped =
                    unsafe { BufferMappedRange::from_raw_parts(ptr.add(*offset) as *mut T, len) };
                Ok(BufferView {
                    mapped,
                    marker: PhantomData,
                })
            }
        }
    }

    pub fn view_mut<T>(&self) -> Result<BufferViewMut<'src, T>, Error> {
        match self {
            BufferSlice::Buffer {
                buffer,
                offset,
                size,
            } => Ok(BufferViewMut {
                mapped: BufferMappedRange::from_buffer(buffer, *offset, size.get())?,
                marker: PhantomData,
            }),
            BufferSlice::Internal { ptr, size, .. } => {
                debug_assert!(
                    size.get() % mem::size_of::<T>() == 0,
                    "Size of the range of the mapped buffer must be multiple of T!"
                );
                let len = size.get() / mem::size_of::<T>();
                let mapped = unsafe { BufferMappedRange::from_raw_parts(*ptr as *mut T, len) };
                Ok(BufferViewMut {
                    mapped,
                    marker: PhantomData,
                })
            }
            BufferSlice::User {
                ptr, offset, size, ..
            } => {
                // TODO(yang): should we allow this?
                debug_assert!(
                    size.get() % mem::size_of::<T>() == 0,
                    "Size of the range of the mapped buffer must be multiple of T!"
                );
                let len = size.get() / mem::size_of::<T>();
                let mapped =
                    unsafe { BufferMappedRange::from_raw_parts(ptr.add(*offset) as *mut T, len) };
                Ok(BufferViewMut {
                    mapped,
                    marker: PhantomData,
                })
            }
        }
    }
}

impl<'src, T> BufferView<'src, T> {
    /// Creates a new slice from the given Buffer with the given offset and
    /// size. Only used internally by [`Buffer::mapped_range`].
    fn new(
        buffer: &'src Buffer,
        offset: usize,
        size: BufferSize,
    ) -> Result<BufferView<'src, T>, Error> {
        Ok(BufferView {
            mapped: BufferMappedRange::from_buffer(buffer, offset, size.into())?,
            marker: PhantomData,
        })
    }
}

impl<'src, T> BufferViewMut<'src, T> {
    /// Creates a new slice from the given Buffer with the given offset and
    /// size. Only used internally by [`Buffer::mapped_range_mut`].
    fn new(
        buffer: &'src Buffer,
        offset: usize,
        size: BufferSize,
    ) -> Result<BufferViewMut<'src, T>, Error> {
        Ok(BufferViewMut {
            mapped: BufferMappedRange::from_buffer(buffer, offset, size.into())?,
            marker: PhantomData,
        })
    }
}

/// A slice of a mapped [`Buffer`].
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
            ptr.add(offset)
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

impl<'src, T> AsRef<[T]> for BufferView<'src, T> {
    fn as_ref(&self) -> &[T] { self.mapped.as_slice() }
}

impl<'src, T> AsMut<[T]> for BufferViewMut<'src, T> {
    fn as_mut(&mut self) -> &mut [T] { self.mapped.as_mut_slice() }
}

impl<'src, T> Deref for BufferView<'src, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target { self.mapped.as_slice() }
}

impl<'src, T> Deref for BufferViewMut<'src, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target { self.mapped.as_slice() }
}

impl<'src, T> DerefMut for BufferViewMut<'src, T> {
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
