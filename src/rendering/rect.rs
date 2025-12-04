use cgmath::*;

use crate::{
    resources::{AppResources, LoadResourceError},
    utils::*,
    wgpu_utils::{
        AsBindGroup, CanvasFormat, IndexBuffer, UniformBuffer, Vertex as _, VertexBuffer,
        vertex_formats::Vertex2dUV,
    },
};

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
    fill_color: UniformBuffer<[f32; 4]>,
}

#[derive(Debug, Clone)]
pub struct RectRenderer<'cx> {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    shader: &'cx wgpu::ShaderModule,
    vertex_buffer: VertexBuffer<Vertex2dUV>,
    index_buffer: IndexBuffer<u16>,
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
                buffers: &[Vertex2dUV::LAYOUT],
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

        // Vertex buffer.
        let vertices_data = &[
            Vertex2dUV::new([0., 0.], [0., 0.]),
            Vertex2dUV::new([1., 0.], [1., 0.]),
            Vertex2dUV::new([1., 1.], [1., 1.]),
            Vertex2dUV::new([0., 1.], [0., 1.]),
        ];
        let vertex_buffer = VertexBuffer::create_init(device, vertices_data);

        // Index buffer.
        let indices_data = &[0u16, 1, 2, 2, 3, 0];
        let index_buffer = IndexBuffer::create_init(device, indices_data);

        Ok(Self {
            bind_group_layout,
            pipeline,
            shader,
            vertex_buffer,
            index_buffer,
        })
    }

    pub fn create_rect(&self, device: &wgpu::Device) -> Rect {
        let bind_group = RectBindGroup {
            model_view: UniformBuffer::create_init(device, Matrix4::identity().into()),
            projection: UniformBuffer::create_init(device, Matrix4::identity().into()),
            fill_color: UniformBuffer::create_init(device, [1., 1., 1., 1.]),
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
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(
            self.index_buffer.slice(..),
            self.index_buffer.index_format(),
        );
        render_pass.draw_indexed(0..6, 0, 0..1);
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

    pub fn set_fill_color(&self, queue: &wgpu::Queue, fill_color: Vector4<f32>) {
        self.bind_group.fill_color.write(fill_color.into(), queue);
    }
}
