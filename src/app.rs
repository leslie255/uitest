use std::sync::Arc;

use cgmath::*;
use pollster::FutureExt as _;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey},
    window::{Window, WindowAttributes, WindowId},
};

use crate::{
    element::{Bounds, RectSize, Texture2d},
    mouse_event::MouseEventRouter,
    resources::AppResources,
    theme::{ButtonKind, Theme},
    utils::*,
    view::{
        ButtonView, ImageView, RectView, SpreadAxis, StackPaddingType, StackView, UiContext, View,
        ViewExt as _, ZStackAlignment, ZStackView, view_lists::*,
    },
    wgpu_utils::{Canvas as _, CanvasView, Srgb, Srgba, WindowCanvas},
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
            mouse_event_router: Arc::new(MouseEventRouter::new(Bounds::default())),
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
            let bounds = Bounds::from_scalars(0., 0., size_logical.width, size_logical.height);
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
    device: wgpu::Device,
    queue: wgpu::Queue,
    window: Arc<Window>,
    window_canvas: WindowCanvas<'static>,
    ui_context: UiContext<'cx, Self>,
    background_rect_view: RectView,
    #[allow(clippy::type_complexity)]
    root_view: Box<dyn View<'cx, Self>>,
}

impl<'cx> UiState<'cx> {
    pub fn create(
        resources: &'cx AppResources,
        window: Arc<Window>,
        event_router: Arc<MouseEventRouter<'cx, Self>>,
    ) -> Self {
        let (instance, adapter, device, queue) = init_wgpu();
        let window_canvas =
            WindowCanvas::create_for_window(&instance, &adapter, &device, window.clone());

        let ui_context = UiContext::create(
            &device,
            &queue,
            resources,
            window_canvas.format(),
            event_router,
        )
        .unwrap_or_else(|e| panic!("{e}"));

        let image_ref = resources.load_image("images/pfp.png").unwrap();
        let texture = Texture2d::create(&device, &queue, image_ref);

        let mut self_ = Self {
            device,
            queue,
            window,
            window_canvas,
            background_rect_view: the_default::<RectView>()
                .with_fill_color(Theme::DEFAULT.primary_background()),
            root_view: StackView::horizontal(ViewList1::new(
                StackView::horizontal(ViewList3::new(
                    ButtonView::new(&ui_context)
                        .with_size(RectSize::new(128., 64.))
                        .with_style(Theme::DEFAULT.button_style(ButtonKind::Mundane).scaled(2.)),
                    ZStackView::new(ViewList2::new(
                        ImageView::new(RectSize::new(100., 100.)).with_texture(texture.clone()),
                        RectView::new(RectSize::new(50., 50.))
                            .with_fill_color(Srgba::from_hex(0x80808080))
                            .with_line_color(Srgb::from_hex(0xFFFFFF))
                            .with_line_width(2.),
                    ))
                    .with_alignment_horizontal(ZStackAlignment::Ratio(0.2))
                    .with_alignment_vertical(ZStackAlignment::Ratio(0.2)),
                    ImageView::new(RectSize::new(100., 100.)).with_texture(texture),
                ))
                .with_padding_type(StackPaddingType::Interpadded)
                .with_fixed_padding(10.),
            ))
            .with_padding_type(StackPaddingType::Omnipadded)
            .with_background_color(Srgb::from_hex(0xFF8080))
            .into_ratio_padded_view()
            .with_ratio_top(0.2)
            .with_ratio_left(0.2)
            .with_background_color(Srgb::from_hex(0xC040FF))
            .into_spread_view(SpreadAxis::Both)
            .into_padded_view()
            .with_padding_top(20.)
            .with_padding_bottom(20.)
            .with_padding_left(80.)
            .with_padding_right(80.)
            .with_background_color(Srgb::from_hex(0x8080FF))
            .into_box_dyn_view(),
            ui_context,
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

        // let seconds = SystemTime::UNIX_EPOCH.elapsed().unwrap().as_secs_f64();
        // let wave = ((f64::sin(seconds * std::f64::consts::TAU / 4.) + 1.) * 0.5) as f32;

        self.ui_context.prepare_view_bounded(
            &self.device,
            &self.queue,
            &canvas,
            canvas.bounds(),
            &mut self.background_rect_view,
        );

        self.ui_context.prepare_view(
            &self.device,
            &self.queue,
            &canvas,
            point2(0., 0.),
            self.root_view.as_mut(),
        );

        self.ui_context
            .draw_view(&mut render_pass, &self.background_rect_view);

        self.ui_context
            .draw_view(&mut render_pass, self.root_view.as_ref());

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
                // self.window.request_redraw();
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } if event.state.is_pressed() => {
                if event.logical_key == Key::Named(NamedKey::F5) {
                    self.window.request_redraw();
                }
            }
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
