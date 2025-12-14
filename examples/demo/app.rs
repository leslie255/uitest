use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey},
    window::{Window, WindowAttributes, WindowId},
};

use crate::theme::Theme;

use muilib::{
    AppResources, Bounds, Canvas as _, CanvasRef, ContainerPadding, EventRouter, ImageView,
    LazyApplicationHandler, RectSize, RectView, Rgba, Srgb, Srgba, StackAlignment, StackView,
    UiContext, View, ViewExt as _, WindowCanvas, ZStackView, view_lists::*,
};

trait OverlayFilter<'cx, UiState: 'cx>: View<'cx, UiState> + Sized {
    fn overlay_filter(self, color: impl Into<Rgba>) -> impl View<'cx, UiState> {
        ZStackView::new(ViewList2::new(
            self,
            RectView::new(RectSize::new(100., 100.))
                .with_fill_color(color)
                .with_line_color(Srgb::from_hex(0xFFFFFF))
                .with_line_width(2.),
        ))
    }
}

impl<'cx, UiState: 'cx, T: View<'cx, UiState>> OverlayFilter<'cx, UiState> for T {}

pub struct App<'cx> {
    window: Arc<Window>,
    window_canvas: WindowCanvas<'static>,
    ui_context: UiContext<'cx, Self>,
    root_view: Box<dyn View<'cx, Self>>,
}

impl<'cx> LazyApplicationHandler<&'cx AppResources> for App<'cx> {
    fn new(resources: &'cx AppResources, event_loop: &ActiveEventLoop) -> Self {
        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default().with_title("UI Test"))
                .unwrap(),
        );
        let window_size_physical = window.inner_size();
        let window_size_logical = window_size_physical.to_logical::<f32>(window.scale_factor());
        let event_router = Arc::new(EventRouter::new(Bounds::from_scalars(
            0.,
            0.,
            window_size_logical.width,
            window_size_logical.height,
        )));
        Self::create(resources, window, event_router)
    }
}

impl<'cx> App<'cx> {
    pub fn create(
        resources: &'cx AppResources,
        window: Arc<Window>,
        event_router: Arc<EventRouter<'cx, Self>>,
    ) -> Self {
        let (ui_context, window_canvas) =
            UiContext::create_for_window(resources, Arc::clone(&window), event_router)
                .unwrap_or_else(|e| panic!("{e}"));

        let theme = &Theme::DEFAULT;

        let image = resources.load_image("images/pfp.png").unwrap();
        let texture = ui_context.create_texture(image);

        let mut self_ = Self {
            window,
            window_canvas,
            root_view: StackView::vertical(ViewList3::new(
                ImageView::new(RectSize::new(100., 100.))
                    .with_texture(texture.clone())
                    .overlay_filter(Srgba::from_hex(0xFF000080)),
                StackView::horizontal(ViewList2::new(
                    ImageView::new(RectSize::new(100., 100.))
                        .with_texture(texture.clone())
                        .overlay_filter(Srgba::from_hex(0x00FF0080)),
                    ImageView::new(RectSize::new(100., 100.))
                        .with_texture(texture.clone())
                        .overlay_filter(Srgba::from_hex(0x0000FF80)),
                ))
                .with_fixed_padding(10.),
                StackView::horizontal(ViewList3::new(
                    ImageView::new(RectSize::new(100., 100.))
                        .with_texture(texture.clone())
                        .overlay_filter(Srgba::from_hex(0xFF00FF80)),
                    ImageView::new(RectSize::new(100., 100.))
                        .with_texture(texture.clone())
                        .overlay_filter(Srgba::from_hex(0x00FFFF80)),
                    ImageView::new(RectSize::new(100., 100.))
                        .with_texture(texture.clone())
                        .overlay_filter(Srgba::from_hex(0xFFFF0080)),
                ))
                .with_fixed_padding(10.),
            ))
            .with_fixed_padding(10.)
            .with_alignment(StackAlignment::Leading)
            .into_container_view()
            .with_padding(ContainerPadding::Fixed(20.))
            .with_background_color(theme.tertiary_background())
            .into_container_view()
            .with_padding(ContainerPadding::Fixed(20.))
            .with_padding(ContainerPadding::Fixed(20.))
            .into_container_view()
            .with_padding_right(ContainerPadding::Spread)
            .with_padding_bottom(ContainerPadding::Spread)
            .with_background_color(theme.secondary_background())
            .into_container_view()
            .with_padding(ContainerPadding::Fixed(20.))
            .with_background_color(theme.primary_background())
            .into_box_dyn_view(),
            ui_context,
        };
        self_.window_resized();
        self_
    }

    fn frame(&mut self, canvas: CanvasRef) {
        let mut render_pass = self
            .ui_context
            .begin_render_pass(&canvas, Srgb::from_hex(0));

        self.ui_context
            .prepare_view_bounded(&canvas, canvas.bounds(), self.root_view.as_mut());
        self.ui_context
            .draw_view(&mut render_pass, self.root_view.as_ref());
    }

    fn window_resized(&mut self) {
        self.window_canvas.reconfigure_for_size(
            self.ui_context.wgpu_device(),
            self.window.inner_size(),
            self.window.scale_factor(),
            None,
        );
    }
}

impl<'cx> ApplicationHandler for App<'cx> {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(_) => self.window_resized(),
            WindowEvent::RedrawRequested => {
                let canvas_view = self.window_canvas.create_ref().unwrap();
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
}
