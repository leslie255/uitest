use std::marker::PhantomData;

use bytemuck::Pod;
use wgpu::util::DeviceExt as _;

#[derive(Debug, Clone)]
pub struct UniformBuffer<T: Pod + Copy> {
    wgpu_buffer: wgpu::Buffer,
    _marker: PhantomData<T>,
}

impl<T: Pod + Copy> UniformBuffer<T> {
    pub fn create_init(device: &wgpu::Device, contents: T) -> Self {
        let bytes = bytemuck::bytes_of(&contents);
        let wgpu_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytes,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            wgpu_buffer,
            _marker: PhantomData,
        }
    }

    pub fn wgpu_buffer(&self) -> &wgpu::Buffer {
        &self.wgpu_buffer
    }

    pub fn wgpu_buffer_mut(&mut self) -> &mut wgpu::Buffer {
        &mut self.wgpu_buffer
    }

    pub fn write(&self, contents: T, queue: &wgpu::Queue) {
        queue.write_buffer(self.wgpu_buffer(), 0, bytemuck::bytes_of(&contents));
    }
}
