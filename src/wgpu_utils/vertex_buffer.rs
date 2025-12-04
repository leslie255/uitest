use std::{marker::PhantomData, ops::RangeBounds};

use bytemuck::{Pod, Zeroable};
use cgmath::*;
use wgpu::util::DeviceExt as _;

pub trait Vertex: Pod + Copy {
    const LAYOUT: wgpu::VertexBufferLayout<'static>;
}

#[derive(Debug, Clone)]
pub struct VertexBuffer<T: Vertex> {
    wgpu_buffer: wgpu::Buffer,
    _marker: PhantomData<T>,
}

impl<T: Vertex> VertexBuffer<T> {
    pub fn create_init(device: &wgpu::Device, contents: &[T]) -> Self {
        let wgpu_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(contents),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::UNIFORM,
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

    pub fn slice<S: RangeBounds<wgpu::BufferAddress>>(&self, bounds: S) -> wgpu::BufferSlice<'_> {
        self.wgpu_buffer.slice(bounds)
    }

    pub fn layout(&self) -> wgpu::VertexBufferLayout<'static> {
        T::LAYOUT
    }
}

pub mod vertex_formats {
    use super::*;

    #[repr(C)]
    #[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
    pub struct Vertex2d {
        pub position: [f32; 2],
    }

    impl From<[f32; 2]> for Vertex2d {
        fn from(value: [f32; 2]) -> Self {
            Self::new(value)
        }
    }

    impl From<Vector2<f32>> for Vertex2d {
        fn from(value: Vector2<f32>) -> Self {
            Self::new(value.into())
        }
    }

    impl Vertex2d {
        pub const fn new(position: [f32; 2]) -> Self {
            Self { position }
        }
    }

    impl Vertex for Vertex2d {
        const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            }],
        };
    }

    #[repr(C)]
    #[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
    pub struct Vertex2dUV {
        pub position: [f32; 2],
        pub uv: [f32; 2],
    }

    impl Vertex2dUV {
        pub const fn new(position: [f32; 2], uv: [f32; 2]) -> Self {
            Self { position, uv }
        }
    }

    impl Vertex for Vertex2dUV {
        const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: size_of::<[f32; 2]>() as u64,
                    shader_location: 1,
                },
            ],
        };
    }

    #[repr(C)]
    #[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
    pub struct Vertex3d {
        pub position: [f32; 3],
    }

    impl Vertex3d {
        pub const fn new(position: [f32; 3]) -> Self {
            Self { position }
        }
    }

    impl From<[f32; 3]> for Vertex3d {
        fn from(value: [f32; 3]) -> Self {
            Self::new(value)
        }
    }

    impl From<Vector3<f32>> for Vertex3d {
        fn from(value: Vector3<f32>) -> Self {
            Self::new(value.into())
        }
    }

    impl Vertex for Vertex3d {
        const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            }],
        };
    }

    #[repr(C)]
    #[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
    pub struct Vertex3dUV {
        pub position: [f32; 3],
        pub uv: [f32; 2],
    }

    impl Vertex3dUV {
        pub const fn new(position: [f32; 3], uv: [f32; 2]) -> Self {
            Self { position, uv }
        }
    }

    impl Vertex for Vertex3dUV {
        const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: size_of::<[f32; 3]>() as u64,
                    shader_location: 1,
                },
            ],
        };
    }

    #[repr(C)]
    #[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
    pub struct Vertex3dUVNormal {
        pub position: [f32; 3],
        pub uv: [f32; 2],
        pub normal: [f32; 3],
    }

    impl Vertex3dUVNormal {
        pub const fn new(position: [f32; 3], uv: [f32; 2], normal: [f32; 3]) -> Self {
            Self {
                position,
                uv,
                normal,
            }
        }
    }

    impl Vertex for Vertex3dUVNormal {
        const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: size_of::<[f32; 5]>() as u64,
                    shader_location: 1,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: size_of::<[f32; 7]>() as u64,
                    shader_location: 2,
                },
            ],
        };
    }
}
