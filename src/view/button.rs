use std::{
    fmt::Debug,
    sync::{
        Arc,
        atomic::{self, AtomicBool, AtomicU8},
    },
};

use cgmath::*;
use winit::event::MouseButton;

use crate::{
    element::{Bounds, LineWidth, RectSize},
    mouse_event::{self, MouseEvent, MouseEventKind, MouseEventListener},
    view::{RectView, TextView, View, ViewContext},
    wgpu_utils::{Srgb, Srgba},
};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ButtonState {
    #[default]
    Idle,
    Hovered,
    Pressed,
    /// Pressed, but have moved outside.
    PressedOutside,
}

#[derive(Debug)]
pub struct AtomicButtonState(AtomicU8);
impl AtomicButtonState {
    pub const fn new(value: ButtonState) -> Self {
        Self(AtomicU8::new(value as u8))
    }

    pub fn load(&self, order: atomic::Ordering) -> ButtonState {
        unsafe { Self::cast_from_repr(self.0.load(order)) }
    }

    pub fn store(&self, value: ButtonState, order: atomic::Ordering) {
        self.0.store(value as u8, order)
    }

    const unsafe fn cast_from_repr(repr: u8) -> ButtonState {
        unsafe { std::mem::transmute(repr) }
    }
}

/// Button style for all `ButtonState`s.
#[derive(Debug, Clone, Copy)]
pub struct ButtonStyle {
    pub line_width: LineWidth,
    pub font_size: f32,
    pub idle_style: ButtonStateStyle,
    pub hovered_style: ButtonStateStyle,
    pub pressed_style: ButtonStateStyle,
}

impl ButtonStyle {
    pub const fn state_style_for(&self, state: ButtonState) -> ButtonStateStyle {
        match state {
            ButtonState::Idle => self.idle_style,
            ButtonState::Hovered => self.hovered_style,
            ButtonState::PressedOutside => self.hovered_style,
            ButtonState::Pressed => self.pressed_style,
        }
    }

    pub fn with_line_width(self, line_width: impl Into<LineWidth>) -> Self {
        Self {
            line_width: line_width.into(),
            ..self
        }
    }

    pub fn with_font_size(self, font_size: f32) -> Self {
        Self { font_size, ..self }
    }
}

/// State-specific button style.
#[derive(Debug, Clone, Copy)]
pub struct ButtonStateStyle {
    pub text_color: Srgb,
    pub fill_color: Srgb,
    pub line_color: Srgb,
}

pub type ButtonCallback<'cx, UiState> =
    Box<dyn for<'a> Fn(&'a mut UiState, ButtonEvent) + Send + Sync + 'cx>;

pub struct ButtonView<'cx, UiState: 'cx> {
    rect_view: RectView,
    text_view: TextView,
    style: ButtonStyle,
    needs_update_bounds: bool,
    dispatch: Arc<ButtonDispatch<'cx, UiState>>,
    listener_handle: mouse_event::ListenerHandle<'cx, UiState>,
}

impl<'cx, UiState> ButtonView<'cx, UiState> {
    pub fn new(
        view_context: &ViewContext<'cx, UiState>,
        style: ButtonStyle,
        callback: Option<ButtonCallback<'cx, UiState>>,
    ) -> Self {
        let default_size = RectSize::new(64., 24.);
        let dispatch = Arc::new(ButtonDispatch {
            state: AtomicButtonState::new(ButtonState::Idle),
            state_updated: AtomicBool::new(true),
            callback,
        });
        let listener_handle = view_context
            .mouse_event_router()
            .register_listener(Bounds::new(point2(0., 0.), default_size), dispatch.clone());
        let mut self_ = Self {
            rect_view: RectView::new(default_size),
            text_view: TextView::new(view_context),
            style,
            needs_update_bounds: true,
            dispatch,
            listener_handle,
        };
        self_.set_style(style);
        self_
    }

    pub fn size(&self) -> RectSize {
        self.rect_view.size()
    }

    pub fn set_size(&mut self, size: impl Into<RectSize>) {
        self.rect_view.set_size(size);
        self.relayout_text();
        self.needs_update_bounds = true;
    }

    pub fn with_size(mut self, size: impl Into<RectSize>) -> Self {
        self.set_size(size);
        self
    }

    pub fn style(&self) -> ButtonStyle {
        self.style
    }

    pub fn set_style(&mut self, style: ButtonStyle) {
        self.style = style;
        self.update_styles();
    }

    pub fn set_title(&mut self, title: String) {
        self.text_view.set_text(title);
        self.relayout_text();
    }

    pub fn state(&self) -> ButtonState {
        self.dispatch.state()
    }

    fn update_styles(&mut self) {
        let style = self.style();
        let state_style = style.state_style_for(self.state());
        self.rect_view.set_fill_color(state_style.fill_color);
        self.rect_view.set_line_color(state_style.line_color);
        self.rect_view.set_line_width(style.line_width);
        if self.text_view.font_size() != style.font_size {
            self.relayout_text();
        }
        self.text_view.set_font_size(style.font_size);
        self.text_view.set_fg_color(state_style.text_color);
        self.text_view.set_bg_color(Srgba::from_hex(0x00000000));
    }

    fn relayout_text(&mut self) {
        let text_size = self.text_view.size();
        let rect_bounds = self.rect_view.bounds();
        let origin = point2(
            rect_bounds.x_min() + 0.5 * (rect_bounds.width() - text_size.width),
            rect_bounds.y_min() + 0.5 * (rect_bounds.height() - text_size.height),
        );
        self.text_view.set_bounds_(Bounds {
            origin,
            size: text_size,
        });
    }
}

impl<'cx, UiState: 'cx> View<UiState> for ButtonView<'cx, UiState> {
    fn preferred_size(&self) -> RectSize {
        self.size()
    }

    fn apply_bounds(&mut self, bounds: Bounds) {
        // Assuming text is single-line.
        self.rect_view.set_bounds_(bounds);
        self.relayout_text();
        self.needs_update_bounds = true;
    }

    fn prepare_for_drawing(
        &mut self,
        view_context: &ViewContext<UiState>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        canvas: &crate::wgpu_utils::CanvasView,
    ) {
        let state_updated = self
            .dispatch
            .state_updated
            .fetch_and(false, atomic::Ordering::AcqRel);
        if state_updated {
            self.update_styles();
        }
        if self.needs_update_bounds {
            self.listener_handle.update_bounds(self.rect_view.bounds());
        }
        self.rect_view
            .prepare_for_drawing(view_context, device, queue, canvas);
        self.text_view
            .prepare_for_drawing(view_context, device, queue, canvas);
    }

    fn draw(&self, view_context: &ViewContext<UiState>, render_pass: &mut wgpu::RenderPass) {
        self.rect_view.draw(view_context, render_pass);
        self.text_view.draw(view_context, render_pass);
    }
}

struct ButtonDispatch<'cx, UiState> {
    state: AtomicButtonState,
    /// Flag for when GPU-side things needs updating after something has changed.
    state_updated: AtomicBool,
    callback: Option<ButtonCallback<'cx, UiState>>,
}

impl<'cx, UiState> ButtonDispatch<'cx, UiState> {
    pub fn state(&self) -> ButtonState {
        self.state.load(atomic::Ordering::Acquire)
    }
}

impl<'cx, UiState> MouseEventListener<UiState> for Arc<ButtonDispatch<'cx, UiState>> {
    fn mouse_event(&self, event: MouseEvent, ui_state: &mut UiState) {
        let old_state = self.state();
        use ButtonState::*;
        use MouseEventKind::*;
        let new_state = match event.kind {
            HoveringStart if old_state == Idle => Hovered,
            HoveringStart if old_state == PressedOutside => Pressed,
            HoveringFinish if old_state == Hovered => Idle,
            HoveringFinish if old_state == Pressed => PressedOutside,
            ButtonDown {
                button: MouseButton::Left,
                started_inside: true,
            } => Pressed,
            ButtonUp {
                button: MouseButton::Left,
                inside: true,
            } => Hovered,
            ButtonUp {
                button: MouseButton::Left,
                inside: false,
            } => Idle,
            _ => old_state,
        };
        self.state.store(new_state, atomic::Ordering::Release);
        self.state_updated.store(true, atomic::Ordering::Release);
        if let Some(callback) = self.callback.as_ref() {
            let button_event = ButtonEvent {
                kind: event.kind,
                position: event.cursor_position,
                previous_state: old_state,
                current_state: new_state,
            };
            callback(ui_state, button_event);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ButtonEvent {
    pub kind: MouseEventKind,
    pub position: Point2<f32>,
    pub previous_state: ButtonState,
    pub current_state: ButtonState,
}

impl ButtonEvent {
    /// Returns true if button event satisfy all of the following:
    ///
    /// - is button up
    /// - is left button
    /// - is inside bounds
    /// - previous state is pressed (so a dragged click starting from
    ///   outside the button and finishing inside does not count)
    pub fn is_button_trigger(self) -> bool {
        self.kind
            == MouseEventKind::ButtonUp {
                button: MouseButton::Left,
                inside: true,
            }
            && self.previous_state == ButtonState::Pressed
    }
}
