use std::{
    array,
    fmt::{self, Debug},
    iter,
    sync::{
        Arc, Mutex, MutexGuard, Weak,
        atomic::{self, AtomicBool, AtomicU64},
    },
};

use cgmath::*;

use winit::event::{MouseButton, WindowEvent};

use crate::{element::Bounds, utils::*};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventKind {
    HoveringStart,
    HoveringFinish,
    ButtonDown {
        button: MouseButton,
        /// `true` if the button is pressed when the cursor is inside the bounds.
        /// `false` if the button is pressed when the cursor is outside the bounds, and is only moved
        /// into the bounds now.
        started_inside: bool,
    },
    ButtonUp {
        button: MouseButton,
        inside: bool,
    },
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
}

pub trait MouseEventListener<UiState>: Send + Sync {
    fn mouse_event(&self, event: MouseEvent, ui_state: &mut UiState);
}

pub struct MouseEventRouter<'cx, UiState> {
    /// `None` if we don't know the position of the cursor.
    cursor_position: Mutex<Option<Point2<f32>>>,
    /// Using `u64` as storage for `f64`, as rust doesn't have an `AtomicF64`.
    scale_factor: AtomicU64,
    bounds: Mutex<Bounds<f32>>,
    listeners: Mutex<Vec<Option<Listener<'cx, UiState>>>>,
    /// Flag for when at least one of the listeners have changed their bounds, indicating that
    /// something in the frame has changed. In this case, we should scan for hovering changes even
    /// if cursor hasn't moved.
    bounds_changed: AtomicBool,
    /// Track states of mouse buttons.
    /// `true` for pressed state.
    button_states: Mutex<[bool; 5]>,
}

impl<'cx, UiState> MouseEventRouter<'cx, UiState> {
    pub fn new(bounds: Bounds<f32>) -> Self {
        Self {
            cursor_position: Mutex::new(None),
            scale_factor: AtomicU64::new(bytemuck::cast(1.0f64)),
            bounds: Mutex::new(bounds),
            listeners: the_default(),
            bounds_changed: AtomicBool::new(false),
            button_states: Mutex::new(array::from_fn(|_| false)),
        }
    }

    pub fn register_listener(
        self: &Arc<Self>,
        bounds: Bounds<f32>,
        listener: impl MouseEventListener<UiState> + 'cx,
    ) -> ListenerHandle<'cx, UiState> {
        let mut listeners = self.listeners.lock().unwrap();
        let index = listeners.len();
        listeners.push(Some(Listener {
            bounds,
            is_hovered: false,
            button_states: array::from_fn(|_| false),
            object: Box::new(listener),
        }));
        ListenerHandle {
            router: Arc::downgrade(self),
            index,
        }
    }

    fn unregister_listener(&self, index: usize) {
        let mut listeners = self.listeners.lock().unwrap();
        listeners[index] = None;
    }

    fn update_bounds(&self, index: usize, bounds: Bounds<f32>) {
        let mut listeners = self.listeners.lock().unwrap();
        listeners[index].as_mut().unwrap().bounds = bounds;
        self.bounds_changed.store(true, atomic::Ordering::Release);
    }

    fn listeners_iter_mut<'a>(
        listeners: &'a mut MutexGuard<Vec<Option<Listener<'cx, UiState>>>>,
    ) -> impl Iterator<Item = &'a mut Listener<'cx, UiState>> + use<'a, 'cx, UiState> {
        listeners.iter_mut().filter_map(Option::as_mut)
    }

    #[allow(dead_code)]
    fn listeners_iter<'a>(
        listeners: &'a MutexGuard<Vec<Option<Listener<'cx, UiState>>>>,
    ) -> impl Iterator<Item = &'a Listener<'cx, UiState>> + use<'a, 'cx, UiState> {
        listeners.iter().filter_map(Option::as_ref)
    }

    /// Returns if should request redraw.
    pub fn window_event(&self, event: &WindowEvent, ui_state: &mut UiState) -> bool {
        _ = ui_state;
        match event {
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                inner_size_writer: _,
            } => {
                self.set_scale_factor(*scale_factor);
                self.scan_events(ui_state)
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                let position_logical = position.to_logical::<f32>(self.get_scale_factor());
                let cursor_position = point2(position_logical.x, position_logical.y);
                self.set_cursor_position(Some(cursor_position));
                self.scan_events(ui_state)
            }
            WindowEvent::CursorLeft { device_id: _ } => {
                self.set_cursor_position(None);
                self.scan_events(ui_state)
            }
            &WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                let index = match button {
                    MouseButton::Left => 0,
                    MouseButton::Right => 1,
                    MouseButton::Middle => 2,
                    MouseButton::Back => 3,
                    MouseButton::Forward => 4,
                    MouseButton::Other(_) => return false,
                };
                let mut button_states = self.button_states.lock().unwrap();
                button_states[index] = state.is_pressed();
                drop(button_states);
                self.scan_events(ui_state)
            }
            WindowEvent::RedrawRequested => {
                let bounds_changed = self
                    .bounds_changed
                    .fetch_and(false, atomic::Ordering::AcqRel);
                if !bounds_changed {
                    return false;
                }
                self.scan_events(ui_state)
            }
            _ => false,
        }
    }

    /// Returns if should redraw.
    fn scan_events(&self, ui_state: &mut UiState) -> bool {
        let Some(cursor_position) = self.get_cursor_position() else {
            return false;
        };
        let mut listeners_locked = self.listeners.lock().unwrap();
        let mut should_redraw = false;
        let button_states = self.button_states.lock().unwrap();
        // Scan for button hovering events.
        for listener in Self::listeners_iter_mut(&mut listeners_locked) {
            let inside = listener.bounds.contains(cursor_position);
            let is_hovered_before = listener.is_hovered;
            // Scan for hovering changes.
            if inside && !listener.is_hovered {
                // Hovering start.
                listener.is_hovered = true;
                listener.object.mouse_event(
                    MouseEvent::new(MouseEventKind::HoveringStart, cursor_position),
                    ui_state,
                );
                should_redraw = true;
            } else if !inside && listener.is_hovered {
                // Hovering finish.
                listener.is_hovered = false;
                listener.object.mouse_event(
                    MouseEvent::new(MouseEventKind::HoveringFinish, cursor_position),
                    ui_state,
                );
                should_redraw = true;
            }
            // Scan for button up/down events.
            // Sanity check in case of future refractors.
            debug_assert!(listener.button_states.len() == button_states.len());
            for (i, (state, listener_state)) in
                iter::zip(button_states.into_iter(), &mut listener.button_states).enumerate()
            {
                let button = match i {
                    0 => MouseButton::Left,
                    1 => MouseButton::Right,
                    2 => MouseButton::Middle,
                    3 => MouseButton::Forward,
                    4 => MouseButton::Back,
                    _ => unreachable!(),
                };
                if !state && *listener_state {
                    // Button up event.
                    *listener_state = state;
                    let event = MouseEvent::new(
                        MouseEventKind::ButtonUp { button, inside },
                        cursor_position,
                    );
                    listener.object.mouse_event(event, ui_state);
                    should_redraw = true;
                } else if state && !*listener_state && inside {
                    // Button down event.
                    *listener_state = state;
                    let started_inside = is_hovered_before;
                    let event = MouseEvent::new(
                        MouseEventKind::ButtonDown {
                            button,
                            started_inside,
                        },
                        cursor_position,
                    );
                    listener.object.mouse_event(event, ui_state);
                    should_redraw = true;
                }
            }
        }
        should_redraw
    }

    pub fn set_bounds(&self, bounds: Bounds<f32>) {
        *self.bounds.lock().unwrap() = bounds;
    }

    pub fn get_bounds(&self) -> Bounds<f32> {
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
    /// The bounds of this listener.
    bounds: Bounds<f32>,
    /// Is the cursor currently hovering over this listener?
    is_hovered: bool,
    /// Records the buttons that the listener is currently being pressed by.
    button_states: [bool; 5],
    /// The listener object type erased and boxed.
    object: Box<dyn MouseEventListener<UiState> + 'cx>,
}

/// Unregisters the listener when dropped.
pub struct ListenerHandle<'cx, UiState> {
    router: Weak<MouseEventRouter<'cx, UiState>>,
    index: usize,
}

impl<'cx, UiState> Clone for ListenerHandle<'cx, UiState> {
    fn clone(&self) -> Self {
        Self {
            router: self.router.clone(),
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
        if let Some(router) = self.router.upgrade() {
            router.unregister_listener(self.index);
        };
    }
}

impl<'cx, UiState> ListenerHandle<'cx, UiState> {
    pub fn update_bounds(&self, bounds: Bounds<f32>) {
        if let Some(router) = self.router.upgrade() {
            router.update_bounds(self.index, bounds);
        };
    }
}
