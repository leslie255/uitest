use bytemuck::{Pod, Zeroable};
use cgmath::*;
use derive_more::From;

use crate::{
    resources::{AppResources, LoadResourceError},
    utils::*,
    wgpu_utils::{AsBindGroup, CanvasFormat, Rgba, UniformBuffer},
};

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BoundingBox {
    pub origin: Point2<f32>,
    pub size: RectSize,
}

impl BoundingBox {
    pub const fn new(x_min: f32, y_min: f32, width: f32, height: f32) -> Self {
        Self {
            origin: point2(x_min, y_min),
            size: RectSize::new(width, height),
        }
    }

    pub const fn x_min(self) -> f32 {
        self.origin.x
    }

    pub const fn y_min(self) -> f32 {
        self.origin.y
    }

    pub const fn x_max(self) -> f32 {
        self.origin.x + self.size.width
    }

    pub const fn y_max(self) -> f32 {
        self.origin.y + self.size.height
    }

    pub const fn end(self) -> Point2<f32> {
        point2(self.x_max(), self.y_max())
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub struct RectSize {
    pub width: f32,
    pub height: f32,
}

impl RectSize {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn as_vec(self) -> Vector2<f32> {
        vec2(self.width, self.height)
    }
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
        top: f32,
        right: f32,
        bottom: f32,
    },
}

impl LineWidth {
    pub const fn to_array(self) -> [f32; 4] {
        match self {
            Self::Uniform(width) => [width, width, width, width],
            Self::PerBorder {
                left,
                top,
                right,
                bottom,
            } => [left, top, right, bottom],
        }
    }

    pub const fn normalized_in(self, size: RectSize) -> Self {
        let [left, top, right, bottom] = self.to_array();
        Self::PerBorder {
            left: left / size.width,
            top: top / size.height,
            right: right / size.width,
            bottom: bottom / size.height,
        }
    }
}

impl From<[f32; 4]> for LineWidth {
    fn from([left, top, right, bottom]: [f32; 4]) -> Self {
        Self::PerBorder {
            left,
            top,
            right,
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

    /// Convenience function over `set_model_view` and `set_line_width`.
    /// Sets `model_view` and `line_width` according to the bounding box and pre-normalized line
    /// width, and then allows applying a transformation matrix on top.
    pub fn set_parameters_with_transform(
        &self,
        queue: &wgpu::Queue,
        bounding_box: BoundingBox,
        line_width: impl Into<LineWidth>,
        model: Matrix4<f32>,
    ) {
        let model_view = model
            * Matrix4::from_translation(bounding_box.origin.to_vec().extend(0.))
            * Matrix4::from_nonuniform_scale(bounding_box.size.width, bounding_box.size.height, 1.);
        self.set_model_view(queue, model_view);
        self.set_line_width(queue, line_width.into().normalized_in(bounding_box.size));
    }

    /// Convenience function over `set_model_view` and `set_line_width`.
    /// Sets `model_view` and `line_width` according to the bounding box and pre-normalized line
    /// width.
    pub fn set_parameters(
        &self,
        queue: &wgpu::Queue,
        bounding_box: BoundingBox,
        line_width: impl Into<LineWidth>,
    ) {
        self.set_parameters_with_transform(queue, bounding_box, line_width, Matrix4::identity());
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
