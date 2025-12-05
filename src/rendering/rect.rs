use bytemuck::{Pod, Zeroable};
use cgmath::*;
use derive_more::From;

use crate::{
    resources::{AppResources, LoadResourceError},
    utils::*,
    wgpu_utils::{AsBindGroup, CanvasFormat, Rgba, UniformBuffer},
};

#[derive(Default, Debug, Clone, Copy, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub struct BoundingBox {
    pub x_min: f32,
    pub y_min: f32,
    pub width: f32,
    pub height: f32,
}

impl BoundingBox {
    pub fn x_max(self) -> f32 {
        self.x_min + self.width
    }

    pub fn y_max(self) -> f32 {
        self.y_min + self.height
    }

    pub fn as_rect_size(self) -> RectSize {
        RectSize {
            width: self.width,
            height: self.height,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub struct RectSize {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, AsBindGroup)]
struct RectBindGroup {
    #[binding(0)]
    #[uniform]
    model_view: UniformBuffer<[[f32; 4]; 4]>,

    #[binding(1)]
    #[uniform]
    projection: UniformBuffer<[[f32; 4]; 4]>,

    #[binding(2)]
    #[uniform]
    fill_color: UniformBuffer<Rgba>,

    #[binding(3)]
    #[uniform]
    line_color: UniformBuffer<Rgba>,

    #[binding(4)]
    #[uniform]
    line_width: UniformBuffer<[f32; 4]>,
}

#[derive(Debug, Clone, Copy, From)]
pub enum LineWidth {
    /// All borders have the same line width.
    Uniform(f32),
    /// Borders have different line widths.
    PerBorder {
        left: f32,
        right: f32,
        top: f32,
        bottom: f32,
    },
}

impl LineWidth {
    pub const fn to_array(self) -> [f32; 4] {
        match self {
            Self::Uniform(width) => [width, width, width, width],
            Self::PerBorder {
                left,
                right,
                top,
                bottom,
            } => [left, right, top, bottom],
        }
    }

    pub fn map(self, mut transform: impl FnMut(f32) -> f32) -> Self {
        match self {
            Self::Uniform(f) => Self::Uniform(transform(f)),
            Self::PerBorder {
                left,
                right,
                top,
                bottom,
            } => Self::PerBorder {
                left: transform(left),
                right: transform(right),
                top: transform(top),
                bottom: transform(bottom),
            },
        }
    }
}

impl From<[f32; 4]> for LineWidth {
    fn from([left, right, top, bottom]: [f32; 4]) -> Self {
        Self::PerBorder {
            left,
            right,
            top,
            bottom,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RectRenderer<'cx> {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    shader: &'cx wgpu::ShaderModule,
}

impl<'cx> RectRenderer<'cx> {
    pub fn create(
        device: &wgpu::Device,
        resources: &'cx AppResources,
        canvas_format: CanvasFormat,
    ) -> Result<Self, LoadResourceError> {
        let shader = resources.load_shader("shaders/rect.wgsl", device)?;
        let bind_group_layout = RectBindGroup::create_bind_group_layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                compilation_options: the_default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("fs_main"),
                compilation_options: the_default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: canvas_format.color_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            operation: wgpu::BlendOperation::Add,
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        },
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: the_default(),
            depth_stencil: canvas_format.depth_stencil_format.map(|format| {
                wgpu::DepthStencilState {
                    format,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Always,
                    stencil: the_default(),
                    bias: the_default(),
                }
            }),
            multisample: the_default(),
            multiview: None,
            cache: None,
        });
        Ok(Self {
            bind_group_layout,
            pipeline,
            shader,
        })
    }

    pub fn create_rect(&self, device: &wgpu::Device) -> Rect {
        let bind_group = RectBindGroup {
            model_view: UniformBuffer::create_init(device, Matrix4::identity().into()),
            projection: UniformBuffer::create_init(device, Matrix4::identity().into()),
            fill_color: UniformBuffer::create_init(device, Rgba::from_hex(0xFFFFFFFF)),
            line_color: UniformBuffer::create_init(device, Rgba::from_hex(0xFFFFFFFF)),
            line_width: UniformBuffer::create_init(device, [0., 0., 0., 0.]),
        };
        let wgpu_bind_group = bind_group.create_bind_group(&self.bind_group_layout, device);
        Rect {
            bind_group,
            wgpu_bind_group,
        }
    }

    pub fn draw_rect(&self, render_pass: &mut wgpu::RenderPass, rect: &Rect) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &rect.wgpu_bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}

#[derive(Debug, Clone)]
pub struct Rect {
    bind_group: RectBindGroup,
    wgpu_bind_group: wgpu::BindGroup,
}

impl Rect {
    pub fn set_model_view(&self, queue: &wgpu::Queue, model_view: Matrix4<f32>) {
        self.bind_group.model_view.write(model_view.into(), queue);
    }

    pub fn set_projection(&self, queue: &wgpu::Queue, projection: Matrix4<f32>) {
        self.bind_group.projection.write(projection.into(), queue);
    }

    pub fn set_fill_color(&self, queue: &wgpu::Queue, fill_color: impl Into<Rgba>) {
        self.bind_group.fill_color.write(fill_color.into(), queue);
    }

    pub fn set_line_color(&self, queue: &wgpu::Queue, line_color: impl Into<Rgba>) {
        self.bind_group.line_color.write(line_color.into(), queue);
    }

    pub fn set_line_width(&self, queue: &wgpu::Queue, line_width: impl Into<LineWidth>) {
        self.bind_group
            .line_width
            .write(line_width.into().to_array(), queue);
    }
}
