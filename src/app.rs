use std::sync::Arc;

use cgmath::*;
use pollster::FutureExt as _;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes, WindowId},
};

use crate::{
    rendering::{Font, Rect, RectRenderer, Text, TextRenderer},
    resources::AppResources,
    utils::*,
};

pub(crate) struct Application<'cx> {
    resources: &'cx AppResources,
    ui: Option<UiState<'cx>>,
}

impl<'cx> Application<'cx> {
    pub fn new(resources: &'cx AppResources) -> Self {
        Self {
            resources,
            ui: None,
        }
    }
}

impl<'cx> ApplicationHandler for Application<'cx> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        match &mut self.ui {
            Some(_) => (),
            ui @ None => {
                let window = event_loop
                    .create_window(WindowAttributes::default().with_title("UI Test"))
                    .unwrap();
                let window = Arc::new(window);
                *ui = Some(UiState::create(self.resources, window));
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(ui) = self.ui.as_mut() {
            ui.window_event(event_loop, window_id, event)
        };
    }
}

fn init_wgpu() -> (wgpu::Instance, wgpu::Adapter, wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .block_on()
        .unwrap();
    let features = wgpu::FeaturesWGPU::POLYGON_MODE_LINE;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: features.into(),
            ..the_default()
        })
        .block_on()
        .unwrap();
    (instance, adapter, device, queue)
}

struct UiState<'cx> {
    resources: &'cx AppResources,
    device: wgpu::Device,
    queue: wgpu::Queue,
    window: Arc<Window>,
    window_surface: wgpu::Surface<'static>,
    window_surface_format: wgpu::TextureFormat,
    size_physical_u: Vector2<u32>,
    size_physical: Vector2<f32>,
    size_logical: Vector2<f32>,
    text_renderer: TextRenderer<'cx>,
    text: Text,
    rect_renderer: RectRenderer<'cx>,
    rect: Rect,
}

impl<'cx> UiState<'cx> {
    pub fn create(resources: &'cx AppResources, window: Arc<Window>) -> Self {
        let (instance, adapter, device, queue) = init_wgpu();
        let window_surface = instance.create_surface(window.retain()).unwrap();
        let surface_capabilities = window_surface.get_capabilities(&adapter);
        let window_surface_format = surface_capabilities
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_capabilities.formats[0]);
        let font = Font::load_from_path(resources, "fonts/big_blue_terminal.json").unwrap();
        let text_renderer = TextRenderer::create(
            &device,
            &queue,
            font,
            resources,
            window_surface_format,
            None,
        )
        .unwrap();
        let text = text_renderer.create_text(&device, "HELLO, WORLD");
        let rect_renderer =
            RectRenderer::create(&device, resources, window_surface_format, None).unwrap();
        let rect = rect_renderer.create_rect(&device);
        let mut self_ = Self {
            resources,
            device,
            queue,
            window,
            window_surface,
            window_surface_format,
            // `window_resized` will update it.
            size_physical_u: Vector2::zero(),
            size_physical: Vector2::zero(),
            size_logical: Vector2::zero(),
            text_renderer,
            text,
            rect_renderer,
            rect,
        };
        self_.window_resized();
        self_
    }

    fn frame(&mut self) {
        let surface_texture = self.window_surface.get_current_texture().unwrap();
        let surface_texture_view = surface_texture.texture.create_view(&the_default());

        let mut encoder = self.device.create_command_encoder(&the_default());

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &surface_texture_view,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
                resolve_target: None,
            })],
            ..the_default()
        });

        let projection = cgmath::ortho(0., self.size_logical.x, self.size_logical.y, 0., -1.0, 1.0);

        // Draw text.
        let model_view_text = Matrix4::from_scale(17.);
        self.text.set_fg_color(&self.queue, vec4(0., 0., 0., 1.));
        self.text.set_bg_color(&self.queue, vec4(0.2, 1., 1., 1.));
        self.text.set_projection(&self.queue, projection);
        self.text.set_model_view(&self.queue, model_view_text);
        self.text_renderer.draw_text(&mut render_pass, &self.text);

        // Draw rect.
        let model_view_rect = Matrix4::from_translation(vec3(100., 200., 0.))
            * Matrix4::from_nonuniform_scale(200., 100., 1.);
        self.rect.set_fill_color(&self.queue, vec4(1., 1., 0.1, 1.));
        self.rect.set_projection(&self.queue, projection);
        self.rect.set_model_view(&self.queue, model_view_rect);
        self.rect_renderer.draw_rect(&mut render_pass, &self.rect);

        drop(render_pass);

        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(_) => self.window_resized(),
            WindowEvent::RedrawRequested => self.frame(),
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => (),
        }
    }

    fn window_resized(&mut self) {
        let window_size = self.window.inner_size();
        self.size_physical_u = vec2(window_size.width, window_size.height);
        self.size_physical = self.size_physical_u.map(|u| u as f32);
        self.size_logical = self
            .size_physical
            .map(|f| f / self.window.scale_factor() as f32);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.window_surface_format,
            view_formats: vec![self.window_surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: window_size.width,
            height: window_size.height,
            desired_maximum_frame_latency: 3,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.window_surface.configure(&self.device, &surface_config);
    }
}
