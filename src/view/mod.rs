use crate::{
    element::{Bounds, RectSize},
    wgpu_utils::CanvasView,
};

mod abstract_views;
mod button;
mod image;
mod rect;
mod stack;
mod text;
mod ui_context;

pub use abstract_views::*;
pub use button::*;
pub use image::*;
pub use rect::*;
pub use stack::*;
pub use text::*;
pub use ui_context::*;

pub mod view_lists;

pub trait View<'cx, UiState>: 'cx {
    fn preferred_size(&mut self) -> RectSize<f32>;
    fn apply_bounds(&mut self, bounds: Bounds<f32>);
    fn prepare_for_drawing(
        &mut self,
        ui_context: &UiContext<'cx, UiState>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        canvas: &CanvasView,
    );
    fn draw(&self, ui_context: &UiContext<'cx, UiState>, render_pass: &mut wgpu::RenderPass);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlow {
    Break,
    Continue,
}

pub trait ViewList<'cx>: 'cx {
    type UiState: 'cx;
    fn for_each_subview(&self, visitor: impl FnMut(&dyn View<'cx, Self::UiState>) -> ControlFlow);
    fn for_each_subview_mut(
        &mut self,
        visitor: impl FnMut(&mut dyn View<'cx, Self::UiState>) -> ControlFlow,
    );
}

#[macro_export]
macro_rules! impl_view_list_ref {
    ( $self:expr, $visitor:expr $(,)? ) => {};
    ( $self:expr, $visitor:expr, $field:ident $(,$($tts:tt)*)? ) => {
        if $visitor(&$self.$field) == $crate::view::ControlFlow::Break {
            return;
        }
        $crate::impl_view_list_ref!($self, $visitor, $($($tts)*)?)
    };
    ( $self:expr, $visitor:expr, $field:ident(iter) $(,$($tts:tt)*)? ) => {
        for subview in &$self.$field {
            if $visitor(subview) == $crate::view::ControlFlow::Break {
                return;
            }
        }
        $crate::impl_view_list_ref!($self, $visitor, $($($tts)*)?)
    };
}

#[macro_export]
macro_rules! impl_view_list_mut {
    ( $self:expr, $visitor:expr $(,)? ) => {};
    ( $self:expr, $visitor:expr, $field:ident $(,$($tts:tt)*)? ) => {
        if $visitor(&mut $self.$field) == $crate::view::ControlFlow::Break {
            return;
        }
        $crate::impl_view_list_mut!($self, $visitor, $($($tts)*)?)
    };
    ( $self:expr, $visitor:expr, $field:ident(iter) $(,$($tts:tt)*)? ) => {
        for subview in &mut $self.$field {
            if $visitor(subview) == $crate::view::ControlFlow::Break {
                return;
            }
        }
        $crate::impl_view_list_mut!($self, $visitor, $($($tts)*)?)
    };
}

#[macro_export]
macro_rules! impl_view_list {
    ($cx:lifetime , $($fields:tt)*) => {
        fn for_each_subview(
            &self,
            mut visitor: impl FnMut(&dyn $crate::view::View<$cx, Self::UiState>) -> $crate::view::ControlFlow,
        ) {
            $crate::impl_view_list_ref!(self, visitor, $($fields)*);
        }
        fn for_each_subview_mut(
            &mut self,
            mut visitor: impl FnMut(&mut dyn $crate::view::View<$cx, Self::UiState>) -> $crate::view::ControlFlow,
        ) {
            $crate::impl_view_list_mut!(self, visitor, $($fields)*);
        }
    };
}
