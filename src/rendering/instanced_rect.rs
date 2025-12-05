use bytemuck::{Pod, Zeroable};
use cgmath::*;

use crate::{
    AppResources,
    rendering::LineWidth,
    resources::LoadResourceError,
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
    ) -> InstancedRects {
        let instance_buffer = VertexBuffer::create_init(device, instances);
        let bind_group = BindGroup {
            projection: UniformBuffer::create_init(device, Matrix4::identity().into()),
        };
        let wgpu_bind_group = bind_group.create_bind_group(&self.bind_group_layout, device);
        InstancedRects {
            bind_group,
            wgpu_bind_group,
            instance_buffer,
            n_instances: instances.len() as u32,
        }
    }

    pub fn draw_rects(&self, render_pass: &mut wgpu::RenderPass, rects: &InstancedRects) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &rects.wgpu_bind_group, &[]);
        render_pass.set_vertex_buffer(0, rects.instance_buffer.slice(..));
        render_pass.draw(0..6, 0..rects.n_instances);
    }
}

#[derive(Debug, Clone)]
pub struct InstancedRects {
    bind_group: BindGroup,
    wgpu_bind_group: wgpu::BindGroup,
    instance_buffer: VertexBuffer<RectInstance>,
    n_instances: u32,
}

impl InstancedRects {
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

#[derive(Debug, Clone, Copy, PartialEq, Zeroable, Pod)]
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
    pub fn new(
        model_view: Matrix3<f32>,
        fill_color: impl Into<Rgba>,
        line_color: impl Into<Rgba>,
        line_width: impl Into<LineWidth>,
    ) -> Self {
        Self {
            model_view_col_0: model_view.x.into(),
            model_view_col_1: model_view.y.into(),
            model_view_col_2: model_view.z.into(),
            fill_color: fill_color.into().to_array(),
            line_color: line_color.into().to_array(),
            line_width: line_width.into().to_array(),
        }
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
