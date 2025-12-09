use std::path::Path;

use std::ops::Range;

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use cgmath::*;

use crate::{
    AppResources,
    element::{Bounds, ImageRef, RectSize, Texture2d},
    resources::LoadResourceError,
    utils::*,
    wgpu_utils::{
        AsBindGroup, CanvasFormat, IndexBuffer, Rgba, UniformBuffer, Vertex, VertexBuffer,
        vertex_formats::Vertex2dUV,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontMetaJson {
    pub path: String,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub glyph_width: u32,
    pub glyph_height: u32,
    pub present_start: u8,
    pub present_end: u8,
    pub glyphs_per_line: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct Font<'cx> {
    present_start: u8,
    present_end: u8,
    glyphs_per_line: u32,
    glyph_size: RectSize<u32>,
    glyph_size_uv: RectSize<f32>,
    atlas_image: ImageRef<'cx>,
}

impl<'cx> Font<'cx> {
    pub fn load_from_resources(
        resources: &'cx AppResources,
        json_subpath: impl AsRef<Path>,
    ) -> Result<Self, LoadResourceError> {
        let json_subpath = json_subpath.as_ref();
        let font_meta = resources.load_json_object::<FontMetaJson>(json_subpath)?;
        let atlas_image_subpath = resources.solve_relative_subpath(json_subpath, &font_meta.path);
        let atlas_image = resources.load_image(&atlas_image_subpath)?;
        Ok(Self {
            present_start: font_meta.present_start,
            present_end: font_meta.present_end,
            glyphs_per_line: font_meta.glyphs_per_line,
            glyph_size: RectSize::new(font_meta.glyph_width, font_meta.glyph_height),
            glyph_size_uv: RectSize::new(
                font_meta.glyph_width as f32 / atlas_image.width() as f32,
                font_meta.glyph_height as f32 / atlas_image.height() as f32,
            ),
            atlas_image: ImageRef {
                width: atlas_image.width(),
                height: atlas_image.height(),
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                data: atlas_image.as_ref(),
            },
        })
    }

    pub fn atlas_image(&self) -> ImageRef<'cx> {
        self.atlas_image
    }

    pub fn present_range(&self) -> Range<u8> {
        self.present_start..self.present_end
    }

    pub fn has_glyph(&self, char: char) -> bool {
        self.present_range().contains(&(char as u8))
    }

    fn uv_position_for_glyph(&self, char: char) -> Option<Point2<f32>> {
        if !self.has_glyph(char) {
            return None;
        }
        let i_glyph = ((char as u8) - self.present_start) as u32;
        let glyph_coord = point2(
            (i_glyph % self.glyphs_per_line) as f32 * self.glyph_size_uv.width,
            (i_glyph / self.glyphs_per_line) as f32 * self.glyph_size_uv.height,
        );
        Some(glyph_coord)
    }

    pub fn uv_bounds_for_char(&self, char: char) -> Option<Bounds<f32>> {
        let top_left = self.uv_position_for_glyph(char)?;
        Some(Bounds::new(top_left, self.glyph_size_uv))
    }

    /// Glyph width if glyph height is 1.
    pub fn glyph_relative_width(&self) -> f32 {
        (self.glyph_size.width as f32) / (self.glyph_size.height as f32)
    }

    pub fn glyph_size(&self) -> RectSize<u32> {
        self.glyph_size
    }

    pub fn glyph_size_uv(&self) -> RectSize<f32> {
        self.glyph_size_uv
    }
}

#[derive(Debug, Clone, AsBindGroup)]
struct TextBindGroup {
    #[binding(0)]
    #[uniform]
    model_view: UniformBuffer<[[f32; 4]; 4]>,

    #[binding(1)]
    #[uniform]
    projection: UniformBuffer<[[f32; 4]; 4]>,

    #[binding(2)]
    #[uniform]
    fg_color: UniformBuffer<[f32; 4]>,

    #[binding(3)]
    #[uniform]
    bg_color: UniformBuffer<[f32; 4]>,

    #[binding(4)]
    #[texture_view]
    texture_view: wgpu::TextureView,

    #[binding(5)]
    #[sampler]
    sampler: wgpu::Sampler,
}

#[derive(Debug, Clone, Copy, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub struct TextInstance {
    pub position_offset: [f32; 2],
    pub uv_offset: [f32; 2],
}

impl Vertex for TextInstance {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<Self>() as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 2,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: size_of::<[f32; 2]>() as u64,
                shader_location: 3,
            },
        ],
    };
}

impl TextInstance {
    pub fn new(position_offset: [f32; 2], uv_offset: [f32; 2]) -> Self {
        Self {
            position_offset,
            uv_offset,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextElement {
    bind_group: TextBindGroup,
    wgpu_bind_group: wgpu::BindGroup,
    n_instances: u32,
    instance_buffer: VertexBuffer<TextInstance>,
}

impl TextElement {
    pub fn set_fg_color(&self, queue: &wgpu::Queue, color: impl Into<Rgba>) {
        self.bind_group
            .fg_color
            .write(color.into().to_array(), queue);
    }

    pub fn set_bg_color(&self, queue: &wgpu::Queue, color: impl Into<Rgba>) {
        self.bind_group
            .bg_color
            .write(color.into().to_array(), queue);
    }

    pub fn set_model_view(&self, queue: &wgpu::Queue, model_view: Matrix4<f32>) {
        self.bind_group.model_view.write(model_view.into(), queue);
    }

    pub fn set_projection(&self, queue: &wgpu::Queue, projection: Matrix4<f32>) {
        self.bind_group.projection.write(projection.into(), queue);
    }

    /// Convenience function over `set_model_view`.
    /// Sets `model_view` according to the bounding box and text size provided.
    pub fn set_parameters(&self, queue: &wgpu::Queue, origin: Point2<f32>, font_size: f32) {
        self.set_model_view(
            queue,
            Matrix4::from_translation(origin.to_vec().extend(0.)) * Matrix4::from_scale(font_size),
        );
    }
}

#[derive(Debug, Clone)]
pub struct TextRenderer<'cx> {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    texture_view: wgpu::TextureView,
    font: Font<'cx>,
    shader: &'cx wgpu::ShaderModule,
    sampler: wgpu::Sampler,
    vertex_buffer: VertexBuffer<Vertex2dUV>,
    index_buffer: IndexBuffer<u16>,
}

impl<'cx> TextRenderer<'cx> {
    pub fn create(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        font: Font<'cx>,
        resources: &'cx AppResources,
        canvas_format: CanvasFormat,
    ) -> Result<Self, LoadResourceError> {
        let shader = resources.load_shader("shaders/text.wgsl", device)?;
        let bind_group_layout = TextBindGroup::create_bind_group_layout(device);
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
                buffers: &[Vertex2dUV::LAYOUT, TextInstance::LAYOUT],
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
        let texture = Texture2d::create(device, queue, font.atlas_image);
        let texture_view = texture.wgpu_texture_view().clone();
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        // Vertex buffer.
        let glyph_size_uv = font.glyph_size_uv();
        let glyph_width = glyph_size_uv.width;
        let glyph_height = glyph_size_uv.height;
        let vertices_data = &[
            Vertex2dUV::new([0., 0.], [0., 0.]),
            Vertex2dUV::new([font.glyph_relative_width(), 0.], [glyph_width, 0.]),
            Vertex2dUV::new(
                [font.glyph_relative_width(), 1.],
                [glyph_width, glyph_height],
            ),
            Vertex2dUV::new([0., 1.], [0., glyph_height]),
        ];
        let vertex_buffer = VertexBuffer::create_init(device, vertices_data);

        // Index buffer.
        let indices_data = &[0u16, 1, 2, 2, 3, 0];
        let index_buffer = IndexBuffer::create_init(device, indices_data);

        Ok(Self {
            bind_group_layout,
            pipeline,
            texture_view,
            font,
            shader,
            sampler,
            vertex_buffer,
            index_buffer,
        })
    }

    pub fn draw_text(&self, render_pass: &mut wgpu::RenderPass, text: &TextElement) {
        if text.n_instances == 0 {
            return;
        }
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &text.wgpu_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, text.instance_buffer.slice(..));
        render_pass.set_index_buffer(
            self.index_buffer.slice(..),
            self.index_buffer.index_format(),
        );
        render_pass.draw_indexed(0..self.index_buffer.length(), 0, 0..text.n_instances);
    }

    pub fn create_text(&self, device: &wgpu::Device, str: &str) -> TextElement {
        let bind_group = TextBindGroup {
            model_view: UniformBuffer::create_init(device, Matrix4::identity().into()),
            projection: UniformBuffer::create_init(device, Matrix4::identity().into()),
            fg_color: UniformBuffer::create_init(device, [1.; 4]),
            bg_color: UniformBuffer::create_init(device, [0.; 4]),
            texture_view: self.texture_view.clone(),
            sampler: self.sampler.clone(),
        };
        let wgpu_bind_group = bind_group.create_bind_group(&self.bind_group_layout, device);
        let (n_instances, instance_buffer) = self.create_instance_buffer(device, str);
        TextElement {
            bind_group,
            wgpu_bind_group,
            n_instances,
            instance_buffer,
        }
    }

    pub fn update_text(&self, device: &wgpu::Device, text: &mut TextElement, str: &str) {
        (text.n_instances, text.instance_buffer) = self.create_instance_buffer(device, str);
    }

    fn create_instance_buffer(
        &self,
        device: &wgpu::Device,
        str: &str,
    ) -> (u32, VertexBuffer<TextInstance>) {
        let mut instances: Vec<TextInstance> = Vec::new();
        let mut row = 0u32;
        let mut column = 0u32;
        for char in str.chars() {
            if char == '\n' {
                column = 0;
                row += 1;
                continue;
            } else if char == '\r' {
                column = 0;
                continue;
            }
            let Some(glyph_bounds) = self.font.uv_bounds_for_char(char) else {
                continue;
            };
            instances.push(TextInstance {
                position_offset: [column as f32 * self.font.glyph_relative_width(), row as f32],
                uv_offset: glyph_bounds.origin.into(),
            });
            column += 1;
        }
        let instance_buffer = VertexBuffer::create_init(device, &instances);
        (instances.len() as u32, instance_buffer)
    }

    pub fn font(&self) -> Font<'cx> {
        self.font
    }
}
