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
    element::{Bounds, LineWidth, RectElement, RectRenderer, TextElement, TextRenderer},
    mouse_event::{self, MouseEvent, MouseEventKind, MouseEventListener, MouseEventRouter},
    view::ViewContext,
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

// pub fn prepare_button_for_drawing(&self, queue: &wgpu::Queue, button: &ButtonView<UiState>) {
//     let style_needs_updating = button
//         .dispatch
//         .needs_updating
//         .fetch_and(false, atomic::Ordering::AcqRel);
//     if style_needs_updating {
//         self.update(queue, button);
//     }
// }

// pub fn draw_button(&self, render_pass: &mut wgpu::RenderPass, button: &ButtonView<UiState>) {
//     self.rect_renderer
//         .draw_rect(render_pass, &button.rect_element);
//     self.text_renderer
//         .draw_text(render_pass, &button.text_element);
// }

// fn update(&self, queue: &wgpu::Queue, button: &ButtonView<UiState>) {
//     let state_style = button.style.state_style_for(button.state());
//     button
//         .rect_element
//         .set_fill_color(queue, state_style.fill_color);
//     button
//         .rect_element
//         .set_line_color(queue, state_style.line_color);
//     button
//         .rect_element
//         .set_parameters(queue, button.rect, state_style.line_width);
//     button
//         .text_element
//         .set_fg_color(queue, state_style.text_color);
//     button
//         .text_element
//         .set_bg_color(queue, Srgba::from_hex(0x00000000));
//     // Assuming text is single-line.
//     let text_height = state_style.font_size;
//     let text_width = (button.title_len as f32)
//         * self.text_renderer.font().glyph_relative_height()
//         * text_height;
//     let top_padding = 0.5 * (button.rect.size.height - text_height);
//     let left_padding = 0.5 * (button.rect.size.width - text_width);
//     let text_origin = point2(
//         button.rect.x_min() + left_padding,
//         button.rect.y_min() + top_padding,
//     );
//     button
//         .text_element
//         .set_parameters(queue, text_origin, state_style.font_size);
// }

// pub type ButtonCallback<'cx, UiState> =
//     Box<dyn for<'a> Fn(&'a mut UiState, MouseEvent) + Send + Sync + 'cx>;
// 
// pub struct ButtonView<'cx, UiState: 'cx> {
//     title_len: usize,
//     bounds: Bounds,
//     rect_element: RectElement,
//     text_element: TextElement,
//     dispatch: Arc<ButtonDispatch<'cx, UiState>>,
//     mouse_listener_handle: mouse_event::ListenerHandle<'cx, UiState>,
//     style: ButtonStyle,
// }
// 
// impl<'cx, UiState> ButtonView<'cx, UiState> {
//     pub fn new(
//         view_context: ViewContext<UiState>,
//         device: &wgpu::Device,
//         title: &str,
//         mouse_event_router: Arc<MouseEventRouter<'cx, UiState>>,
//     ) -> Self {
//         let rect_element = view_context.rect_renderer.create_rect(device);
//         let text_element = view_context.text_renderer.create_text(device, title);
//         let dispatch = Arc::new(ButtonDispatch {
//             state: AtomicButtonState::new(ButtonState::Idle),
//             needs_updating: true.into(),
//             callback: None,
//         });
//         let mouse_listener_handle = view_context
//             .mouse_event_router
//             .register_listener(rect, dispatch.clone());
//         ButtonView {
//             title_len: title.len(),
//             bounds: rect,
//             rect_element,
//             text_element,
//             dispatch,
//             mouse_listener_handle,
//             style,
//         }
//     }
// 
//     pub fn set_projection(&self, queue: &wgpu::Queue, projection: Matrix4<f32>) {
//         self.rect_element.set_projection(queue, projection);
//         self.text_element.set_projection(queue, projection);
//     }
// 
//     pub fn bounding_box(&self) -> Bounds {
//         self.bounds
//     }
// 
//     pub fn state(&self) -> ButtonState {
//         self.dispatch.state()
//     }
// }
// 
// struct ButtonDispatch<'cx, UiState> {
//     state: AtomicButtonState,
//     /// Flag for when GPU-side things needs updating after something has changed.
//     needs_updating: AtomicBool,
//     callback: Option<ButtonCallback<'cx, UiState>>,
// }
// 
// impl<'cx, UiState> ButtonDispatch<'cx, UiState> {
//     pub fn state(&self) -> ButtonState {
//         self.state.load(atomic::Ordering::Acquire)
//     }
// 
//     pub fn set_state(&self, state: ButtonState) {
//         self.state.store(state, atomic::Ordering::Release);
//     }
// }
// 
// impl<'cx, UiState> MouseEventListener<UiState> for Arc<ButtonDispatch<'cx, UiState>> {
//     fn mouse_event(&self, event: MouseEvent, ui_state: &mut UiState) {
//         let old_state = self.state();
//         use ButtonState::*;
//         use MouseEventKind::*;
//         let new_state = match event.kind {
//             HoveringStart if old_state == Idle => Hovered,
//             HoveringStart if old_state == PressedOutside => Pressed,
//             HoveringFinish if old_state == Hovered => Idle,
//             HoveringFinish if old_state == Pressed => PressedOutside,
//             ButtonDown {
//                 button: MouseButton::Left,
//             } => Pressed,
//             ButtonUp {
//                 button: MouseButton::Left,
//                 inside: true,
//             } => Hovered,
//             ButtonUp {
//                 button: MouseButton::Left,
//                 inside: false,
//             } => Idle,
//             _ => old_state,
//         };
//         self.set_state(new_state);
//         self.needs_updating
//             .fetch_or(old_state != new_state, atomic::Ordering::AcqRel);
//         if let Some(callback) = self.callback.as_ref() {
//             callback(ui_state, event)
//         }
//     }
// }
