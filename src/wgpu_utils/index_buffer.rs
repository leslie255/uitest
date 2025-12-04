use std::{marker::PhantomData, ops::RangeBounds};

use bytemuck::Pod;
use wgpu::util::DeviceExt as _;

pub trait Index: Pod + Copy {
    const FORMAT: wgpu::IndexFormat;
}

impl Index for u16 {
    const FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint16;
}

impl Index for u32 {
    const FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint32;
}

#[derive(Debug, Clone)]
pub struct IndexBuffer<T: Index> {
    wgpu_buffer: wgpu::Buffer,
    length: u32,
    _marker: PhantomData<T>,
}

impl<T: Index> IndexBuffer<T> {
    pub fn create_init(device: &wgpu::Device, contents: &[T]) -> Self {
        let wgpu_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(contents),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::UNIFORM,
        });
        Self {
            wgpu_buffer,
            length: contents.len().try_into().unwrap(),
            _marker: PhantomData,
        }
    }

    pub fn wgpu_buffer(&self) -> &wgpu::Buffer {
        &self.wgpu_buffer
    }

    pub fn wgpu_buffer_mut(&mut self) -> &mut wgpu::Buffer {
        &mut self.wgpu_buffer
    }

    pub fn slice<S: RangeBounds<wgpu::BufferAddress>>(&self, bounds: S) -> wgpu::BufferSlice<'_> {
        self.wgpu_buffer.slice(bounds)
    }

    pub fn index_format(&self) -> wgpu::IndexFormat {
        T::FORMAT
    }

    pub fn length(&self) -> u32 {
        self.length
    }

    /// This is always safe because wgpu is safe.
    pub fn length_mut(&mut self) -> &mut u32 {
        &mut self.length
    }
}
