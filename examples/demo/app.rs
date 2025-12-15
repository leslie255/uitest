use std::sync::Arc;

use muilib::{Canvas as _, RectSize, Srgb};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey},
    window::{Window, WindowAttributes, WindowId},
};

use crate::theme::Theme;

pub struct App<'cx> {
    window: Arc<Window>,
    window_canvas: muilib::WindowCanvas<'static>,
    ui_context: muilib::UiContext<'cx, Self>,
    rects: Vec<muilib::RectView>,
}

impl<'cx> muilib::LazyApplicationHandler<&'cx muilib::AppResources> for App<'cx> {
    fn new(resources: &'cx muilib::AppResources, event_loop: &ActiveEventLoop) -> Self {
        let window = event_loop
            .create_window(WindowAttributes::default().with_title("UI Test"))
            .unwrap();
        Self::create(resources, window)
    }
}

impl<'cx> App<'cx> {
    pub fn create(resources: &'cx muilib::AppResources, window: Window) -> Self {
        let window = Arc::new(window);
        let event_router = Arc::new(muilib::EventRouter::new());
        let (ui_context, window_canvas) =
            muilib::UiContext::create_for_window(resources, window.clone(), event_router.clone())
                .unwrap_or_else(|e| panic!("{e}"));

        // let image = resources.load_image("images/pfp.png").unwrap();
        // let texture = ui_context.create_texture(image);

        let colors = [0x0000C0, 0x00C000, 0xC00000, 0x008080, 0x808000, 0x800080];
        let mut self_ = Self {
            window,
            window_canvas,
            rects: colors
                .into_iter()
                .map(|color| {
                    muilib::RectView::new(RectSize::new(100., 100.))
                        .with_fill_color(Srgb::from_hex(color))
                        .with_line_color(Srgb::from_hex(0xFFFFFF))
                        .with_line_width(2.)
                })
                .collect(),
            ui_context,
        };
        self_.window_resized();
        self_
    }

    fn frame(&mut self, canvas: muilib::CanvasRef) {
        let layout = self.ui_context.begin_layout_pass();

        let [row0, row1, row2] = self.rects.get_disjoint_mut([0..1, 1..3, 3..6]).unwrap();
        let root_view = layout.vstack(|vstack| {
            vstack.set_fixed_padding(10.);
            vstack.set_alignment_vertical(muilib::StackAlignmentVertical::Bottom);
            vstack.set_alignment_horizontal(muilib::StackAlignmentHorizontal::Right);
            vstack.subview(layout.hstack(|hstack| {
                hstack.set_fixed_padding(10.);
                for rect in row0 {
                    hstack.subview(rect);
                }
            }));
            vstack.subview(layout.hstack(|hstack| {
                hstack.set_fixed_padding(10.);
                for rect in row1 {
                    hstack.subview(rect);
                }
            }));
            vstack.subview(layout.hstack(|hstack| {
                hstack.set_fixed_padding(10.);
                for rect in row2 {
                    hstack.subview(rect);
                }
            }));
        });

        let root_view_packaged = layout
            .container(root_view)
            .override_subview_size(RectSize::new(f32::INFINITY, f32::INFINITY))
            .set_padding(muilib::ContainerPadding::Fixed(20.));

        self.ui_context
            .prepare_view_bounded(&canvas, canvas.bounds(), root_view_packaged);

        let mut render_pass = self
            .ui_context
            .begin_render_pass(&canvas, Theme::DEFAULT.primary_background());

        self.ui_context.draw_view(&mut render_pass, root_view);
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
        let should_redraw = self.ui_context.event_router().window_event(&event, self);
        if should_redraw {
            self.window.request_redraw();
        }
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
