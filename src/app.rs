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
    rendering::{
        Font, InstancedRectRenderer, InstancedRects, LineWidth, Rect, RectInstance, RectRenderer,
        Text, TextRenderer,
    },
    resources::AppResources,
    utils::*,
    wgpu_utils::{Canvas as _, CanvasView, ProjectionSpace, Srgb, WindowCanvas},
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
    let instance = wgpu::Instance::new(&the_default());
    let adapter = instance.request_adapter(&the_default()).block_on().unwrap();
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
    window_canvas: WindowCanvas<'static>,
    text_renderer: TextRenderer<'cx>,
    text: Text,
    rect_renderer: RectRenderer<'cx>,
    rect_background: Rect,
    rect: Rect,
    instanced_rects_renderer: InstancedRectRenderer<'cx>,
    instanced_rects: InstancedRects,
}

impl<'cx> UiState<'cx> {
    pub fn create(resources: &'cx AppResources, window: Arc<Window>) -> Self {
        let (instance, adapter, device, queue) = init_wgpu();
        let window_canvas = WindowCanvas::create_for_window(
            &instance,
            &adapter,
            &device,
            window.retain(),
            |color_format| wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: color_format,
                view_formats: vec![color_format],
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                width: window.inner_size().width,
                height: window.inner_size().height,
                desired_maximum_frame_latency: 3,
                present_mode: wgpu::PresentMode::AutoVsync,
            },
        );
        let canvas_format = window_canvas.format();
        let font = Font::load_from_path(resources, "fonts/big_blue_terminal.json").unwrap();
        let text_renderer =
            TextRenderer::create(&device, &queue, font, resources, canvas_format).unwrap();
        let text = text_renderer.create_text(&device, "HELLO, WORLD");
        let rect_renderer = RectRenderer::create(&device, resources, canvas_format).unwrap();
        let rect = rect_renderer.create_rect(&device);
        let rect_background = rect_renderer.create_rect(&device);
        let instanced_rects_renderer =
            InstancedRectRenderer::create(&device, resources, canvas_format).unwrap();
        let instanced_rects = instanced_rects_renderer.create_rects(
            &device,
            &[
                RectInstance::new(
                    Matrix3::from_translation(vec2(100., 400.)) * Matrix3::from_scale(100.),
                    Srgb::from_hex(0x008080),
                    Srgb::from_hex(0xFFFFFF),
                    std::array::from_fn(|i| (i as f32 + 1.) * 4. / 100.),
                ),
                RectInstance::new(
                    Matrix3::from_translation(vec2(300., 400.)) * Matrix3::from_scale(120.),
                    Srgb::from_hex(0x800080),
                    Srgb::from_hex(0xFFFFFF),
                    std::array::from_fn(|i| (i as f32 + 1.) * 4. / 120.),
                ),
                RectInstance::new(
                    Matrix3::from_translation(vec2(520., 400.)) * Matrix3::from_scale(160.),
                    Srgb::from_hex(0x808000),
                    Srgb::from_hex(0xFFFFFF),
                    std::array::from_fn(|i| (i as f32 + 1.) * 4. / 120.),
                ),
            ],
        );
        let mut self_ = Self {
            resources,
            device,
            queue,
            window,
            window_canvas,
            text_renderer,
            text,
            rect_renderer,
            rect,
            rect_background,
            instanced_rects_renderer,
            instanced_rects,
        };
        self_.window_resized();
        self_
    }

    fn frame(&mut self, canvas: CanvasView) {
        assert!(
            canvas.depth_stencil_texture_view.is_none(),
            "TODO: drawing with depth stencil buffer"
        );
        let mut encoder = self.device.create_command_encoder(&the_default());
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &canvas.color_texture_view,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
                resolve_target: None,
            })],
            ..the_default()
        });

        let projection = canvas.projection(ProjectionSpace::TopLeftDown, -1.0, 1.0);

        // Draw background rect.
        self.rect_background
            .set_fill_color(&self.queue, Srgb::from_hex(0x303030));
        self.rect_background.set_model_view(
            &self.queue,
            Matrix4::from_translation(vec3(-1.0, -1.0, 0.0)) * Matrix4::from_scale(2.0),
        );
        self.rect_renderer
            .draw_rect(&mut render_pass, &self.rect_background);

        // Draw rect.
        let rect_width = 400.;
        let rect_height = 200.;
        let rect_line_width = 4.;
        let model_view_rect = Matrix4::from_translation(vec3(20., 20., 0.))
            * Matrix4::from_nonuniform_scale(rect_width, rect_height, 1.);
        self.rect
            .set_fill_color(&self.queue, Srgb::from_hex(0xFBC000));
        self.rect
            .set_line_color(&self.queue, Srgb::from_hex(0xFFFFFF));
        self.rect.set_projection(&self.queue, projection);
        self.rect.set_model_view(&self.queue, model_view_rect);
        self.rect.set_line_width(
            &self.queue,
            LineWidth::PerBorder {
                left: 4. / rect_width,
                right: 4. / rect_width,
                top: 4. / rect_height,
                bottom: 4. / rect_height,
            },
        );
        self.rect_renderer.draw_rect(&mut render_pass, &self.rect);

        // Draw text.
        let model_view_text =
            Matrix4::from_translation(vec3(20. + rect_line_width, 20. + rect_line_width, 0.))
                * Matrix4::from_scale(29.);
        self.text
            .set_fg_color(&self.queue, Srgb::from_hex(0xFFFFFF));
        self.text
            .set_bg_color(&self.queue, Srgb::from_hex(0x008080));
        self.text.set_projection(&self.queue, projection);
        self.text.set_model_view(&self.queue, model_view_text);
        self.text_renderer.draw_text(&mut render_pass, &self.text);

        // Draw instanced rects.
        self.instanced_rects.set_projection(&self.queue, projection);
        self.instanced_rects_renderer
            .draw_rects(&mut render_pass, &self.instanced_rects);

        drop(render_pass);

        self.queue.submit([encoder.finish()]);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(_) => self.window_resized(),
            WindowEvent::RedrawRequested => {
                let canvas_view = self.window_canvas.begin_drawing().unwrap();
                self.frame(canvas_view);
                self.window.pre_present_notify();
                self.window_canvas.finish_drawing().unwrap();
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => (),
        }
    }

    fn window_resized(&mut self) {
        self.window_canvas.reconfigure_for_size(
            &self.device,
            self.window.inner_size(),
            self.window.scale_factor(),
            None,
        );
    }
}
