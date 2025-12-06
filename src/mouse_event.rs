use std::{
    fmt::{self, Debug},
    sync::{
        Arc, Mutex, Weak,
        atomic::{self, AtomicU64},
    },
};

use cgmath::*;

use winit::event::{ElementState, MouseButton, WindowEvent};

use crate::{shapes::BoundingBox, utils::*};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventKind {
    HoveringStart,
    HoveringFinish,
    ButtonDown { button: MouseButton },
    ButtonUp { button: MouseButton, inside: bool },
}

#[derive(Debug, Clone, Copy)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub cursor_position: Point2<f32>,
}

impl MouseEvent {
    pub fn new(kind: MouseEventKind, cursor_position: Point2<f32>) -> Self {
        Self {
            kind,
            cursor_position,
        }
    }

    /// Returns true if mouse event:
    ///
    /// - is left button, and ...
    /// - is button up, and ...
    /// - is inside bounds
    pub fn is_button_trigger(self) -> bool {
        matches!(
            self.kind,
            MouseEventKind::ButtonUp {
                button: MouseButton::Left,
                inside: true
            }
        )
    }
}

pub trait MouseEventListener<'cx, UiState: 'cx>: 'cx + Send + Sync {
    fn mouse_event(&self, event: MouseEvent, ui_state: &mut UiState);
}

pub struct MouseEventRouter<'cx, UiState> {
    /// `None` if we don't know the position of the cursor.
    cursor_position: Mutex<Option<Point2<f32>>>,
    /// Using `u64` as storage for `f64`, as rust doesn't have an `AtomicF64`.
    scale_factor: AtomicU64,
    bounds: Mutex<BoundingBox>,
    listeners: Mutex<Vec<Listener<'cx, UiState>>>,
}

impl<'cx, UiState> MouseEventRouter<'cx, UiState> {
    pub fn new(bounds: BoundingBox) -> Self {
        Self {
            cursor_position: Mutex::new(None),
            scale_factor: AtomicU64::new(bytemuck::cast(1.0f64)),
            bounds: Mutex::new(bounds),
            listeners: the_default(),
        }
    }

    pub fn register_listener(
        self: &Arc<Self>,
        bounds: BoundingBox,
        listener: impl MouseEventListener<'cx, UiState>,
    ) -> ListenerHandle<'cx, UiState> {
        let mut listeners = self.listeners.lock().unwrap();
        let index = listeners.len();
        listeners.push(Listener {
            bounds,
            is_hovered: false,
            is_pressed: false,
            object: Box::new(listener),
        });
        ListenerHandle {
            router_inner: Arc::downgrade(self),
            index,
        }
    }

    fn unregister_listener(&self, index: usize) {
        let mut listeners = self.listeners.lock().unwrap();
        listeners.remove(index);
    }

    /// Returns if should request redraw.
    pub fn window_event(&self, event: &WindowEvent, ui_state: &mut UiState) -> bool {
        let mut should_redraw = false;
        _ = ui_state;
        match event {
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                inner_size_writer: _,
            } => {
                self.set_scale_factor(*scale_factor);
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                let position_logical = position.to_logical::<f32>(self.get_scale_factor());
                let cursor_position = point2(position_logical.x, position_logical.y);
                self.set_cursor_position(Some(cursor_position));
                let mut listeners = self.listeners.lock().unwrap();
                for listener in listeners.iter_mut() {
                    if listener.bounds.contains(cursor_position) && !listener.is_hovered {
                        listener.is_hovered = true;
                        listener.object.mouse_event(
                            MouseEvent::new(MouseEventKind::HoveringStart, cursor_position),
                            ui_state,
                        );
                        should_redraw = true;
                    }
                    if !listener.bounds.contains(cursor_position) && listener.is_hovered {
                        listener.is_hovered = false;
                        listener.object.mouse_event(
                            MouseEvent::new(MouseEventKind::HoveringFinish, cursor_position),
                            ui_state,
                        );
                        should_redraw = true;
                    }
                }
            }
            WindowEvent::CursorLeft { device_id: _ } => {
                self.set_cursor_position(None);
            }
            &WindowEvent::MouseInput {
                device_id: _,
                state: ElementState::Pressed,
                button,
            } => {
                let Some(cursor_position) = self.get_cursor_position() else {
                    return should_redraw;
                };
                let mut listeners = self.listeners.lock().unwrap();
                for listener in listeners.iter_mut() {
                    if !listener.bounds.contains(cursor_position) {
                        continue;
                    }
                    listener.is_pressed = true;
                    listener.object.mouse_event(
                        MouseEvent::new(MouseEventKind::ButtonDown { button }, cursor_position),
                        ui_state,
                    );
                    should_redraw = true;
                }
            }
            &WindowEvent::MouseInput {
                device_id: _,
                state: ElementState::Released,
                button,
            } => {
                let Some(cursor_position) = self.get_cursor_position() else {
                    return should_redraw;
                };
                let mut listeners = self.listeners.lock().unwrap();
                for listener in listeners.iter_mut() {
                    if !listener.is_pressed {
                        continue;
                    }
                    let inside = listener.bounds.contains(cursor_position);
                    listener.is_pressed = false;
                    listener.object.mouse_event(
                        MouseEvent::new(
                            MouseEventKind::ButtonUp { button, inside },
                            cursor_position,
                        ),
                        ui_state,
                    );
                    should_redraw = true;
                }
            }
            _ => (),
        }
        should_redraw
    }

    pub fn set_bounds(&self, bounding_box: BoundingBox) {
        *self.bounds.lock().unwrap() = bounding_box;
    }

    pub fn get_bounds(&self) -> BoundingBox {
        *self.bounds.lock().unwrap()
    }

    fn get_scale_factor(&self) -> f64 {
        let u = self.scale_factor.load(atomic::Ordering::Relaxed);
        bytemuck::cast(u)
    }

    fn set_scale_factor(&self, scale_factor: f64) {
        let u = bytemuck::cast(scale_factor);
        self.scale_factor.store(u, atomic::Ordering::Relaxed);
    }

    fn get_cursor_position(&self) -> Option<Point2<f32>> {
        *self.cursor_position.lock().unwrap()
    }

    /// Returns old value.
    fn set_cursor_position(&self, cursor_position: Option<Point2<f32>>) -> Option<Point2<f32>> {
        let mut cursor_position_ = self.cursor_position.lock().unwrap();
        *cursor_position_ = cursor_position;
        *cursor_position_
    }
}

struct Listener<'cx, UiState> {
    bounds: BoundingBox,
    is_hovered: bool,
    is_pressed: bool,
    object: Box<dyn MouseEventListener<'cx, UiState>>,
}

/// Unregisters the listener when dropped.
pub struct ListenerHandle<'cx, UiState> {
    router_inner: Weak<MouseEventRouter<'cx, UiState>>,
    index: usize,
}

impl<'cx, UiState> Clone for ListenerHandle<'cx, UiState> {
    fn clone(&self) -> Self {
        Self {
            router_inner: self.router_inner.clone(),
            index: self.index,
        }
    }
}

impl<UiState> Debug for ListenerHandle<'_, UiState> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ListenerHandle")
            .field("index", &self.index)
            .finish_non_exhaustive()
    }
}

impl<UiState> Drop for ListenerHandle<'_, UiState> {
    fn drop(&mut self) {
        let Some(router_inner) = self.router_inner.upgrade() else {
            return;
        };
        router_inner.unregister_listener(self.index);
    }
}
