use crate::{
    element::{Bounds, RectSize},
    wgpu_utils::CanvasView,
};

mod button;
mod rect;
mod stack;
mod text;
mod view_context;

pub use button::*;
pub use rect::*;
pub use stack::*;
pub use text::*;
pub use view_context::*;

pub trait View<UiState> {
    fn preferred_size(&self) -> RectSize;
    fn apply_bounds(&mut self, bounds: Bounds);
    fn prepare_for_drawing(
        &mut self,
        view_context: &ViewContext<UiState>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        canvas: &CanvasView,
    );
    fn draw(&self, view_context: &ViewContext<UiState>, render_pass: &mut wgpu::RenderPass);
}
