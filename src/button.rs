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
    mouse_event::{self, MouseEvent, MouseEventKind, MouseEventListener, MouseEventRouter},
    shapes::{BoundingBox, LineWidth, Rect, RectRenderer, Text, TextRenderer},
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
        let line_width = line_width.into();
        Self {
            idle_style: ButtonStateStyle {
                line_width,
                ..self.idle_style
            },
            hovered_style: ButtonStateStyle {
                line_width,
                ..self.hovered_style
            },
            pressed_style: ButtonStateStyle {
                line_width,
                ..self.pressed_style
            },
        }
    }

    pub fn with_font_size(self, font_size: f32) -> Self {
        Self {
            idle_style: ButtonStateStyle {
                font_size,
                ..self.idle_style
            },
            hovered_style: ButtonStateStyle {
                font_size,
                ..self.hovered_style
            },
            pressed_style: ButtonStateStyle {
                font_size,
                ..self.pressed_style
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ButtonStateStyle {
    pub line_width: LineWidth,
    pub font_size: f32,
    pub text_color: Srgb,
    pub fill_color: Srgb,
    pub line_color: Srgb,
}

#[derive(Clone)]
pub struct ButtonRenderer<'cx, UiState: 'cx> {
    text_renderer: TextRenderer<'cx>,
    rect_renderer: RectRenderer<'cx>,
    mouse_event_router: Arc<MouseEventRouter<'cx, UiState>>,
}

impl<'cx, UiState: 'cx> ButtonRenderer<'cx, UiState> {
    pub fn new(
        text_renderer: TextRenderer<'cx>,
        rect_renderer: RectRenderer<'cx>,
        mouse_event_router: Arc<MouseEventRouter<'cx, UiState>>,
    ) -> Self {
        Self {
            text_renderer,
            rect_renderer,
            mouse_event_router,
        }
    }

    pub fn create_button(
        &self,
        device: &wgpu::Device,
        bounding_box: BoundingBox,
        style: ButtonStyle,
        title: &str,
        callback: Option<ButtonCallback<'cx, UiState>>,
    ) -> Button<'cx, UiState> {
        let rect = self.rect_renderer.create_rect(device);
        let text = self.text_renderer.create_text(device, title);
        let dispatch = Arc::new(ButtonDispatch {
            state: AtomicButtonState::new(ButtonState::Idle),
            needs_updating: true.into(),
            callback,
        });
        let mouse_listener_handle = self
            .mouse_event_router
            .register_listener(bounding_box, dispatch.clone());
        Button {
            title_len: title.len(),
            bounding_box,
            rect,
            text,
            dispatch,
            mouse_listener_handle,
            style,
        }
    }

    pub fn prepare_button_for_drawing(&self, queue: &wgpu::Queue, button: &Button<UiState>) {
        let style_needs_updating = button
            .dispatch
            .needs_updating
            .fetch_and(false, atomic::Ordering::AcqRel);
        if style_needs_updating {
            self.update(queue, button);
        }
    }

    pub fn draw_button(&self, render_pass: &mut wgpu::RenderPass, button: &Button<UiState>) {
        self.rect_renderer.draw_rect(render_pass, &button.rect);
        self.text_renderer.draw_text(render_pass, &button.text);
    }

    fn update(&self, queue: &wgpu::Queue, button: &Button<UiState>) {
        let state_style = button.style.state_style_for(button.state());
        button.rect.set_fill_color(queue, state_style.fill_color);
        button.rect.set_line_color(queue, state_style.line_color);
        button
            .rect
            .set_parameters(queue, button.bounding_box, state_style.line_width);
        button.text.set_fg_color(queue, state_style.text_color);
        button.text.set_bg_color(queue, Srgba::from_hex(0x00000000));
        // Assuming text is single-line.
        let text_height = state_style.font_size;
        let text_width = (button.title_len as f32)
            * self.text_renderer.font().glyph_relative_height()
            * text_height;
        let top_padding = 0.5 * (button.bounding_box.size.height - text_height);
        let left_padding = 0.5 * (button.bounding_box.size.width - text_width);
        let text_origin = point2(
            button.bounding_box.x_min() + left_padding,
            button.bounding_box.y_min() + top_padding,
        );
        button
            .text
            .set_parameters(queue, text_origin, state_style.font_size);
    }
}

pub type ButtonCallback<'cx, UiState> =
    Box<dyn for<'a> Fn(&'a mut UiState, MouseEvent) + Send + Sync + 'cx>;

pub struct Button<'cx, UiState: 'cx> {
    title_len: usize,
    bounding_box: BoundingBox,
    rect: Rect,
    text: Text,
    dispatch: Arc<ButtonDispatch<'cx, UiState>>,
    mouse_listener_handle: mouse_event::ListenerHandle<'cx, UiState>,
    style: ButtonStyle,
}

impl<'cx, UiState> Button<'cx, UiState> {
    pub fn set_projection(&self, queue: &wgpu::Queue, projection: Matrix4<f32>) {
        self.rect.set_projection(queue, projection);
        self.text.set_projection(queue, projection);
    }

    pub fn bounding_box(&self) -> BoundingBox {
        self.bounding_box
    }

    pub fn state(&self) -> ButtonState {
        self.dispatch.state()
    }
}

struct ButtonDispatch<'cx, UiState> {
    state: AtomicButtonState,
    /// Flag for when GPU-side things needs updating after something has changed.
    needs_updating: AtomicBool,
    callback: Option<ButtonCallback<'cx, UiState>>,
}

impl<'cx, UiState> ButtonDispatch<'cx, UiState> {
    pub fn state(&self) -> ButtonState {
        self.state.load(atomic::Ordering::Acquire)
    }

    pub fn set_state(&self, state: ButtonState) {
        self.state.store(state, atomic::Ordering::Release);
    }
}

impl<'cx, UiState: 'cx> MouseEventListener<'cx, UiState> for Arc<ButtonDispatch<'cx, UiState>> {
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
        self.set_state(new_state);
        self.needs_updating
            .fetch_or(old_state != new_state, atomic::Ordering::AcqRel);
        if let Some(callback) = self.callback.as_ref() {
            callback(ui_state, event)
        }
    }
}
