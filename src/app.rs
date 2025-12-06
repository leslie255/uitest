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
    button::{Button, ButtonRenderer},
    mouse_event::MouseEventRouter,
    resources::AppResources,
    shapes::{BoundingBox, Font, Rect, RectRenderer, TextRenderer},
    theme::{ButtonKind, Theme},
    utils::*,
    wgpu_utils::{Canvas as _, CanvasView, ProjectionSpace, WindowCanvas},
};

pub(crate) struct Application<'cx> {
    resources: &'cx AppResources,
    mouse_event_router: Arc<MouseEventRouter<'cx, UiState<'cx>>>,
    window: Option<Arc<Window>>,
    ui: Option<UiState<'cx>>,
}

impl<'cx> Application<'cx> {
    pub fn new(resources: &'cx AppResources) -> Self {
        Self {
            resources,
            mouse_event_router: Arc::new(MouseEventRouter::new(BoundingBox::default())),
            window: None,
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
                self.window = Some(Arc::clone(&window));
                *ui = Some(UiState::create(
                    self.resources,
                    window,
                    self.mouse_event_router.clone(),
                ));
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        if window_id != window.id() {
            return;
        }
        if let WindowEvent::Resized(size_physical) = event {
            let size_logical = size_physical.to_logical::<f32>(window.scale_factor());
            let bounds = BoundingBox::new(0., 0., size_logical.width, size_logical.height);
            self.mouse_event_router.set_bounds(bounds);
        }
        if let Some(ui) = self.ui.as_mut() {
            let should_redraw = self.mouse_event_router.window_event(&event, ui);
            if should_redraw {
                window.request_redraw();
            }
            ui.window_event(event_loop, window_id, event);
        }
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
    rect_renderer: RectRenderer<'cx>,
    rect_background: Rect,
    button_renderer: ButtonRenderer<'cx, UiState<'cx>>,
    button_mundane: Button<'cx, UiState<'cx>>,
    button_primary: Button<'cx, UiState<'cx>>,
    button_toxic: Button<'cx, UiState<'cx>>,
    counter: i64,
    counter_text: crate::shapes::Text,
}

impl<'cx> UiState<'cx> {
    pub fn create(
        resources: &'cx AppResources,
        window: Arc<Window>,
        event_router: Arc<MouseEventRouter<'cx, Self>>,
    ) -> Self {
        let (instance, adapter, device, queue) = init_wgpu();
        let window_canvas = WindowCanvas::create_for_window(
            &instance,
            &adapter,
            &device,
            window.clone(),
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

        let rect_renderer = RectRenderer::create(&device, resources, canvas_format).unwrap();

        let rect_background = rect_renderer.create_rect(&device);
        rect_background.set_fill_color(&queue, Theme::DEFAULT.primary_background());
        rect_background.set_parameters(&queue, BoundingBox::new(-1., -1., 2., 2.), 0.);

        let font = Font::load_from_path(resources, "fonts/big_blue_terminal.json").unwrap();
        let text_renderer =
            TextRenderer::create(&device, &queue, font, resources, canvas_format).unwrap();

        let counter_text = text_renderer.create_text(&device, "0");
        counter_text.set_parameters(&queue, point2(20., 20.), 24.);

        let button_renderer =
            ButtonRenderer::new(text_renderer.clone(), rect_renderer.clone(), event_router);

        let width = 128.;
        let height = 48.;
        let inter_padding = 10.;
        let y_offset = 54.0f32;
        let bounding_box = |i: usize| -> BoundingBox {
            BoundingBox::new(
                20. + (i as f32) * (width + inter_padding),
                y_offset,
                width,
                height,
            )
        };
        let button_mundane = {
            button_renderer.create_button(
                &device,
                bounding_box(0),
                Theme::DEFAULT
                    .button_style(ButtonKind::Mundane)
                    .with_font_size(24.)
                    .with_line_width(4.),
                "-1",
                Some(Box::new(|self_, event| {
                    if event.is_button_trigger() {
                        self_.counter -= 1;
                        self_.update_counter_text();
                    }
                })),
            )
        };
        let button_primary = {
            button_renderer.create_button(
                &device,
                bounding_box(1),
                Theme::DEFAULT
                    .button_style(ButtonKind::Primary)
                    .with_font_size(24.)
                    .with_line_width(4.),
                "+1",
                Some(Box::new(|self_, event| {
                    if event.is_button_trigger() {
                        self_.counter += 1;
                        self_.update_counter_text();
                    }
                })),
            )
        };
        let button_toxic = {
            button_renderer.create_button(
                &device,
                bounding_box(2),
                Theme::DEFAULT
                    .button_style(ButtonKind::Toxic)
                    .with_font_size(24.)
                    .with_line_width(4.),
                "SET 0",
                Some(Box::new(|self_, event| {
                    if event.is_button_trigger() {
                        self_.counter = 0;
                        self_.update_counter_text();
                    }
                })),
            )
        };

        let mut self_ = Self {
            resources,
            device,
            queue,
            window,
            window_canvas,
            text_renderer,
            rect_renderer,
            rect_background,
            button_renderer,
            button_mundane,
            button_primary,
            button_toxic,
            counter: 0,
            counter_text,
        };
        self_.window_resized();
        self_
    }

    pub fn update_counter_text(&mut self) {
        self.text_renderer.update_text(
            &self.device,
            &mut self.counter_text,
            &format!("{}", self.counter),
        );
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
        self.rect_renderer
            .draw_rect(&mut render_pass, &self.rect_background);

        // Draw text.
        self.counter_text.set_projection(&self.queue, projection);
        self.text_renderer
            .draw_text(&mut render_pass, &self.counter_text);

        // Draw button.
        self.button_mundane.set_projection(&self.queue, projection);
        self.button_renderer
            .prepare_button_for_drawing(&self.queue, &self.button_mundane);
        self.button_renderer
            .draw_button(&mut render_pass, &self.button_mundane);

        self.button_primary.set_projection(&self.queue, projection);
        self.button_renderer
            .prepare_button_for_drawing(&self.queue, &self.button_primary);
        self.button_renderer
            .draw_button(&mut render_pass, &self.button_primary);

        self.button_toxic.set_projection(&self.queue, projection);
        self.button_renderer
            .prepare_button_for_drawing(&self.queue, &self.button_toxic);
        self.button_renderer
            .draw_button(&mut render_pass, &self.button_toxic);

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
