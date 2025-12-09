use bytemuck::{Pod, Zeroable};
use cgmath::*;

use crate::{
    AppResources,
    resources::LoadResourceError,
    element::{Bounds, LineWidth},
    utils::*,
    wgpu_utils::{AsBindGroup, CanvasFormat, Rgba, UniformBuffer, Vertex, VertexBuffer},
};

#[derive(Debug, Clone)]
pub struct InstancedRectRenderer<'cx> {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    shader: &'cx wgpu::ShaderModule,
}

impl<'cx> InstancedRectRenderer<'cx> {
    pub fn create(
        device: &wgpu::Device,
        resources: &'cx AppResources,
        canvas_format: CanvasFormat,
    ) -> Result<Self, LoadResourceError> {
        let shader = resources.load_shader("shaders/instanced_rect.wgsl", device)?;
        let bind_group_layout = BindGroup::create_bind_group_layout(device);
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
                buffers: &[RectInstance::LAYOUT],
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

    pub fn create_rects(
        &self,
        device: &wgpu::Device,
        instances: &[RectInstance],
    ) -> InstancedRectsElement {
        let instance_buffer = VertexBuffer::create_init(device, instances);
        let bind_group = BindGroup {
            projection: UniformBuffer::create_init(device, Matrix4::identity().into()),
        };
        let wgpu_bind_group = bind_group.create_bind_group(&self.bind_group_layout, device);
        InstancedRectsElement {
            bind_group,
            wgpu_bind_group,
            instance_buffer,
            n_instances: instances.len() as u32,
        }
    }

    pub fn draw_rects(&self, render_pass: &mut wgpu::RenderPass, rects: &InstancedRectsElement) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &rects.wgpu_bind_group, &[]);
        render_pass.set_vertex_buffer(0, rects.instance_buffer.slice(..));
        render_pass.draw(0..6, 0..rects.n_instances);
    }
}

#[derive(Debug, Clone)]
pub struct InstancedRectsElement {
    bind_group: BindGroup,
    wgpu_bind_group: wgpu::BindGroup,
    instance_buffer: VertexBuffer<RectInstance>,
    n_instances: u32,
}

impl InstancedRectsElement {
    pub fn set_projection(&self, queue: &wgpu::Queue, projection: Matrix4<f32>) {
        self.bind_group.projection.write(projection.into(), queue);
    }
}

#[derive(Debug, Clone, AsBindGroup)]
struct BindGroup {
    #[binding(0)]
    #[uniform]
    projection: UniformBuffer<[[f32; 4]; 4]>,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub struct RectInstance {
    model_view_col_0: [f32; 3],
    model_view_col_1: [f32; 3],
    model_view_col_2: [f32; 3],
    fill_color: [f32; 4],
    line_color: [f32; 4],
    line_width: [f32; 4],
}

impl RectInstance {
    pub fn new() -> Self {
        Self::zeroed()
    }

    pub fn with_model_view(self, model_view: Matrix3<f32>) -> Self {
        Self {
            model_view_col_0: model_view.x.into(),
            model_view_col_1: model_view.y.into(),
            model_view_col_2: model_view.z.into(),
            ..self
        }
    }

    pub fn with_normalized_line_width(self, line_width: impl Into<LineWidth>) -> Self {
        Self {
            line_width: line_width.into().to_array(),
            ..self
        }
    }

    pub fn with_fill_color(self, fill_color: impl Into<Rgba>) -> Self {
        Self {
            fill_color: fill_color.into().to_array(),
            ..self
        }
    }

    pub fn with_line_color(self, line_color: impl Into<Rgba>) -> Self {
        Self {
            line_color: line_color.into().to_array(),
            ..self
        }
    }

    /// Convenience function over `with_model_view` and `with_normalized_line_width`.
    /// Sets `model_view` and normalized `line_width` according to the bounds and line width
    /// provided.
    pub fn from_parameters(rect: Bounds<f32>, line_width: impl Into<LineWidth>) -> Self {
        let model_view = Matrix3::from_translation(rect.origin.to_vec())
            * Matrix3::from_nonuniform_scale(rect.size.width, rect.size.height);
        let line_width_normalized = line_width.into().normalized_in(rect.size);
        Self::new()
            .with_model_view(model_view)
            .with_normalized_line_width(line_width_normalized)
    }
}

impl Vertex for RectInstance {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<Self>() as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array! [
            0 => Float32x3, // model_view_col_0
            1 => Float32x3, // model_view_col_1
            2 => Float32x3, // model_view_col_2
            3 => Float32x4, // fill_color
            4 => Float32x4, // line_color
            5 => Float32x4, // line_width
        ],
    };
}
