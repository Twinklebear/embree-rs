use std::os::raw::c_uint;
use std::marker::PhantomData;
use std::mem;
use std::cell::Ref;
use std::ops::{Index, IndexMut};

use sys::*;
use scene::Scene;

pub struct Buffer<'a, T> {
    scene: &'a Scene<'a>,
    geom: c_uint,
    len: usize,
    buffer_type: BufferType,
    marker: PhantomData<T>
}

impl<'a, T> Buffer<'a, T> {
    /// TODO: How to deal with attachable buffers?
    pub(crate) fn new(scene: &'a Scene<'a>, geom: c_uint, len: usize,
                      buffer_type: BufferType) -> Buffer<'a, T> {
        Buffer {
            scene: scene,
            geom: geom,
            len: len,
            buffer_type: buffer_type,
            marker: PhantomData
        }
    }
    pub fn map<'b>(&'b mut self) -> MappedBuffer<'b, T> {
        let slice = unsafe {
            rtcMapBuffer(*self.scene.handle.borrow(), self.geom,
                         mem::transmute(self.buffer_type)) as *mut T
        };
        MappedBuffer {
            scene: self.scene.handle.borrow(),
            geom: self.geom,
            buffer_type: self.buffer_type,
            slice: slice,
            len: self.len
        }
    }
}

pub struct MappedBuffer<'a, T: 'a> {
    scene: Ref<'a, RTCScene>,
    geom: u32,
    buffer_type: BufferType,
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

impl<'a, T: 'a> Drop for MappedBuffer<'a, T> {
    fn drop(&mut self) {
        unsafe {
            rtcUnmapBuffer(*self.scene, self.geom, mem::transmute(self.buffer_type));
        }
    }
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BufferType {
    IndexBuffer = 16777216,
    IndexBuffer1 = 16777217,
    VertexBuffer = 33554432,
    VertexBuffer1 = 33554433,
    UserVertexBuffer = 34603008,
    UserVertexBuffer1 = 34603009,
    FaceBuffer = 50331648,
    LevelBuffer = 67108865,
    EdgeCreaseIndexBuffer = 83886080,
    EdgeCreaseWeightBuffer = 100663296,
    VertexCreaseIndexBuffer = 117440512,
    VertexCreaseWeightBuffer = 134217728,
    HoleBuffer = 150994945,
}

