use std::fmt::{self, Debug};

use cgmath::*;
use derive_more::From;

use crate::{
    resources::{AppResources, LoadResourceError},
    utils::*,
    wgpu_utils::{AsBindGroup, CanvasFormat, Rgba, UniformBuffer},
};

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Bounds<T: Copy> {
    pub origin: Point2<T>,
    pub size: RectSize<T>,
}

impl<T: Copy + Debug> Debug for Bounds<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Bounds")
            .field("x_min", &self.x_min())
            .field("y_min", &self.y_min())
            .field("width", &self.width())
            .field("height", &self.height())
            .finish()
    }
}

impl<T: Copy> Default for Bounds<T>
where
    T: Zero,
{
    fn default() -> Self {
        Self {
            origin: point2(T::zero(), T::zero()),
            size: the_default(),
        }
    }
}

impl<T: Copy> Bounds<T> {
    pub const fn new(origin: Point2<T>, size: RectSize<T>) -> Self {
        Self { origin, size }
    }

    pub const fn from_scalars(x_min: T, y_min: T, width: T, height: T) -> Self {
        Self {
            origin: point2(x_min, y_min),
            size: RectSize::new(width, height),
        }
    }

    pub const fn x_min(self) -> T {
        self.origin.x
    }

    pub const fn y_min(self) -> T {
        self.origin.y
    }

    pub const fn width(self) -> T {
        self.size.width
    }

    pub const fn height(self) -> T {
        self.size.height
    }

    pub const fn with_origin(self, origin: Point2<T>) -> Self {
        Self { origin, ..self }
    }

    pub const fn with_size(self, size: RectSize<T>) -> Self {
        Self { size, ..self }
    }
}

impl Bounds<f32> {
    pub const fn x_max(self) -> f32 {
        self.origin.x + self.size.width
    }

    pub const fn y_max(self) -> f32 {
        self.origin.y + self.size.height
    }

    pub const fn xy_max(self) -> Point2<f32> {
        point2(self.x_max(), self.y_max())
    }

    pub const fn xy_min(self) -> Point2<f32> {
        self.origin
    }

    pub const fn contains(self, point: Point2<f32>) -> bool {
        self.x_min() <= point.x
            && point.x <= self.x_max()
            && self.y_min() <= point.y
            && point.y <= self.y_max()
    }

    pub const fn with_padding(self, padding: f32) -> Self {
        Self::from_scalars(
            self.x_min() + padding,
            self.y_min() + padding,
            self.width() - padding - padding,
            self.height() - padding - padding,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RectSize<T: Copy> {
    pub width: T,
    pub height: T,
}

impl<T: Copy> Default for RectSize<T>
where
    T: Zero,
{
    fn default() -> Self {
        Self {
            width: T::zero(),
            height: T::zero(),
        }
    }
}

impl<T: Copy> RectSize<T> {
    pub const fn new(width: T, height: T) -> Self {
        Self { width, height }
    }

    pub const fn as_vec(self) -> Vector2<T> {
        vec2(self.width, self.height)
    }
}

impl RectSize<f32> {
    /// `min` per-axis.
    pub fn min(self, other: Self) -> Self {
        Self {
            width: self.width.min(other.width),
            height: self.height.min(other.height),
        }
    }

    /// `min` per-axis.
    pub fn max(self, other: Self) -> Self {
        Self {
            width: self.width.max(other.width),
            height: self.height.max(other.height),
        }
    }

    pub const fn scaled(self, scale_horizontal: f32, scale_vertical: f32) -> Self {
        Self {
            width: self.width * scale_horizontal,
            height: self.height * scale_vertical,
        }
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

impl Default for LineWidth {
    fn default() -> Self {
        Self::Uniform(0.)
    }
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

    pub const fn normalized_in(self, size: RectSize<f32>) -> Self {
        let [left, top, right, bottom] = self.to_array();
        Self::PerBorder {
            left: left / size.width,
            top: top / size.height,
            right: right / size.width,
            bottom: bottom / size.height,
        }
    }

    pub const fn left(&self) -> f32 {
        match self {
            LineWidth::Uniform(width) => *width,
            LineWidth::PerBorder { left, .. } => *left,
        }
    }

    pub const fn top(&self) -> f32 {
        match self {
            LineWidth::Uniform(width) => *width,
            LineWidth::PerBorder { top, .. } => *top,
        }
    }

    pub const fn right(&self) -> f32 {
        match self {
            LineWidth::Uniform(width) => *width,
            LineWidth::PerBorder { right, .. } => *right,
        }
    }

    pub const fn bottom(&self) -> f32 {
        match self {
            LineWidth::Uniform(width) => *width,
            LineWidth::PerBorder { bottom, .. } => *bottom,
        }
    }

    pub const fn set_left(&mut self, left_width: f32) {
        let [_, top, right, bottom] = self.to_array();
        *self = Self::PerBorder {
            left: left_width,
            top,
            right,
            bottom,
        };
    }

    pub const fn set_top(&mut self, top_width: f32) {
        let [left, _, right, bottom] = self.to_array();
        *self = Self::PerBorder {
            left,
            top: top_width,
            right,
            bottom,
        };
    }

    pub const fn set_right(&mut self, right_width: f32) {
        let [left, top, _, bottom] = self.to_array();
        *self = Self::PerBorder {
            left,
            top,
            right: right_width,
            bottom,
        };
    }

    pub const fn set_bottom(&mut self, bottom_width: f32) {
        let [left, top, right, _] = self.to_array();
        *self = Self::PerBorder {
            left,
            top,
            right,
            bottom: bottom_width,
        };
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
    _shader: &'cx wgpu::ShaderModule,
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
            _shader: shader,
        })
    }

    pub fn create_rect(&self, device: &wgpu::Device) -> RectElement {
        let bind_group = RectBindGroup {
            model_view: UniformBuffer::create_init(device, Matrix4::identity().into()),
            projection: UniformBuffer::create_init(device, Matrix4::identity().into()),
            fill_color: UniformBuffer::create_init(device, Rgba::from_hex(0xFFFFFFFF)),
            line_color: UniformBuffer::create_init(device, Rgba::from_hex(0xFFFFFFFF)),
            line_width: UniformBuffer::create_init(device, [0., 0., 0., 0.]),
        };
        let wgpu_bind_group = bind_group.create_bind_group(&self.bind_group_layout, device);
        RectElement {
            bind_group,
            wgpu_bind_group,
        }
    }

    pub fn draw_rect(&self, render_pass: &mut wgpu::RenderPass, rect: &RectElement) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &rect.wgpu_bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}

#[derive(Debug, Clone)]
pub struct RectElement {
    bind_group: RectBindGroup,
    wgpu_bind_group: wgpu::BindGroup,
}

impl RectElement {
    pub fn set_model_view(&self, queue: &wgpu::Queue, model_view: Matrix4<f32>) {
        self.bind_group.model_view.write(model_view.into(), queue);
    }

    pub fn set_projection(&self, queue: &wgpu::Queue, projection: Matrix4<f32>) {
        self.bind_group.projection.write(projection.into(), queue);
    }

    /// Convenience function over `set_model_view` and `set_normalized_line_width`.
    /// Sets `model_view` and normalized `line_width` according to the bounds and line width
    /// provided.
    pub fn set_parameters(
        &self,
        queue: &wgpu::Queue,
        bounds: Bounds<f32>,
        line_width: impl Into<LineWidth>,
    ) {
        let model_view = Matrix4::from_translation(bounds.origin.to_vec().extend(0.))
            * Matrix4::from_nonuniform_scale(bounds.size.width, bounds.size.height, 1.);
        self.set_model_view(queue, model_view);
        self.set_normalized_line_width(queue, line_width.into().normalized_in(bounds.size));
    }

    pub fn set_fill_color(&self, queue: &wgpu::Queue, fill_color: impl Into<Rgba>) {
        self.bind_group.fill_color.write(fill_color.into(), queue);
    }

    pub fn set_line_color(&self, queue: &wgpu::Queue, line_color: impl Into<Rgba>) {
        self.bind_group.line_color.write(line_color.into(), queue);
    }

    pub fn set_normalized_line_width(&self, queue: &wgpu::Queue, line_width: impl Into<LineWidth>) {
        self.bind_group
            .line_width
            .write(line_width.into().to_array(), queue);
    }
}
