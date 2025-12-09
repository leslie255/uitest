use std::{sync::Arc, time::SystemTime};

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
    element::{Bounds, ImageRef, RectSize, Texture2d},
    impl_view_list,
    mouse_event::MouseEventRouter,
    resources::AppResources,
    theme::{ButtonKind, Theme},
    utils::*,
    view::{
        ButtonEvent, ButtonView, HStackView, ImageView, RectView, SpreadView,
        StackLayout, TextView, UiContext, VStackView, ViewList, view_lists::*,
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

struct HStack0<'cx> {
    rect_views: Vec<RectView>,
    button_view: ButtonView<'cx, UiState<'cx>>,
}
impl<'cx> ViewList<'cx> for HStack0<'cx> {
    type UiState = UiState<'cx>;
    impl_view_list! {
        'cx,
        rect_views(iter),
        button_view,
    }
}

impl<'cx> HStack0<'cx> {
    pub fn new(ui_context: &UiContext<'cx, UiState<'cx>>) -> HStackView<'cx, Self> {
        let colors = [0x008080, 0x404080, 0xB04020];
        let line_width = 2.;
        HStackView::new(Self {
            rect_views: colors
                .into_iter()
                .map(|color| {
                    RectView::new(RectSize::new(64., 64.))
                        .with_fill_color(Srgb::from_hex(color))
                        .with_line_color(Srgb::from_hex(0xFFFFFF))
                        .with_line_width(line_width)
                })
                .collect(),
            button_view: {
                ButtonView::new(ui_context)
                    .with_callback(UiState::button_callback_increment)
                    .with_style(Theme::DEFAULT.button_style(ButtonKind::Primary).scaled(2.))
                    .with_size(RectSize::new(128., 48.))
                    .with_title(String::from("+1"))
            },
        })
        .with_inter_padding(10.)
    }
}

struct HStack2<'cx> {
    image_view: ImageView,
    button_view: ButtonView<'cx, UiState<'cx>>,
    text_view: TextView<'cx>,
}
impl<'cx> ViewList<'cx> for HStack2<'cx> {
    type UiState = UiState<'cx>;
    impl_view_list! {
        'cx,
        image_view,
        button_view,
        text_view,
    }
}

impl<'cx> HStack2<'cx> {
    pub fn new(
        ui_context: &UiContext<'cx, UiState<'cx>>,
        texture: Texture2d,
    ) -> HStackView<'cx, Self> {
        HStackView::new(Self {
            image_view: ImageView::new_with_texture(texture).with_size(RectSize::new(28., 28.)),
            button_view: {
                ButtonView::new(ui_context)
                    .with_callback(UiState::button_callback_decrement)
                    .with_style(Theme::DEFAULT.button_style(ButtonKind::Mundane).scaled(2.))
                    .with_size(RectSize::new(128., 48.))
                    .with_title(String::from("-1"))
            },
            text_view: TextView::new(ui_context)
                .with_font_size(48.)
                .with_fg_color(Srgb::from_hex(0xFFFFFF))
                .with_bg_color(Srgb::from_hex(0x308050)),
        })
        .with_inter_padding(10.)
    }
}

struct UiState<'cx> {
    resources: &'cx AppResources,
    device: wgpu::Device,
    queue: wgpu::Queue,
    window: Arc<Window>,
    window_canvas: WindowCanvas<'static>,
    ui_context: UiContext<'cx, Self>,
    background_rect_view: RectView,
    counter: i64,
    #[allow(clippy::type_complexity)]
    stack: SpreadView<
        VStackView<
            'cx,
            ViewList3<
                Self,
                SpreadView<HStackView<'cx, HStack0<'cx>>>,
                SpreadView<HStackView<'cx, ViewList1<Self, TextView<'cx>>>>,
                SpreadView<HStackView<'cx, HStack2<'cx>>>,
            >,
        >,
    >,
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

        let image = resources.load_image("images/pfp.png").unwrap();
        let image_ref = ImageRef {
            width: image.width(),
            height: image.height(),
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            data: image.as_ref(),
        };
        let texture = Texture2d::create(&device, &queue, image_ref);

        let mut self_ = Self {
            resources,
            device,
            queue,
            window,
            window_canvas,
            background_rect_view: the_default::<RectView>()
                .with_fill_color(Theme::DEFAULT.primary_background()),
            counter: 0,
            stack: SpreadView::vertical(
                VStackView::new(ViewList3::new(
                    SpreadView::horizontal(HStack0::new(&ui_context)),
                    SpreadView::horizontal(HStackView::new(ViewList1::new(
                        TextView::new(&ui_context)
                            .with_font_size(48.0)
                            .with_bg_color(Srgba::from_hex(0x00000000))
                            .with_fg_color(Srgb::from_hex(0xFFFFFF)),
                    ))),
                    SpreadView::horizontal(HStack2::new(&ui_context, texture)),
                ))
                .with_layout(StackLayout::EqualSpacing),
            ),
            ui_context,
        };
        self_.window_resized();
        self_.update_counter_text();
        self_
    }

    fn button_callback_increment(&mut self, event: ButtonEvent) {
        if event.is_button_trigger() {
            self.counter += 1;
            self.update_counter_text();
        }
    }

    fn button_callback_decrement(&mut self, event: ButtonEvent) {
        if event.is_button_trigger() {
            self.counter -= 1;
            self.update_counter_text();
        }
    }

    fn update_counter_text(&mut self) {
        let text_view = &mut self.stack.subviews_mut().view1.subviews_mut().view0;
        text_view.set_text(format!("{}", self.counter));
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

        let seconds = SystemTime::UNIX_EPOCH.elapsed().unwrap().as_secs_f64();
        let wave = ((f64::sin(seconds * std::f64::consts::TAU / 4.) + 1.) * 0.5) as f32;

        for rect_view in &mut self.stack.subviews_mut().view0.subviews_mut().rect_views {
            let min_width = rect_view.line_width().left() + rect_view.line_width().right();
            rect_view.size_mut().width = (64. - min_width) * wave + min_width;
        }

        self.stack
            .subviews_mut()
            .view2
            .subviews_mut()
            .text_view
            .set_text({
                let wave_u = (wave * 12.).round() as usize;
                let mut string = String::with_capacity(wave_u);
                for _ in 0..wave_u {
                    string.push('A');
                }
                string
            });

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
            &mut self.stack,
        );

        self.ui_context
            .draw_view(&mut render_pass, &self.background_rect_view);

        self.ui_context.draw_view(&mut render_pass, &self.stack);

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
                self.window.request_redraw();
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
