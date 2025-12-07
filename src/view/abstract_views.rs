use derive_more::{AsMut, AsRef, Deref, DerefMut};

use crate::{
    element::{Bounds, RectSize},
    param_getters_setters,
    view::View,
};

use super::ViewContext;

/// An empty view for just leaving a bit of space empty.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct SpacerView {
    size: RectSize,
}

impl SpacerView {
    pub const fn new(size: RectSize) -> Self {
        Self { size }
    }

    param_getters_setters! {
        vis: pub,
        param_ty: RectSize,
        param: size,
        param_mut: size_mut,
        set_param: set_size,
        with_param: with_size,
        param_mut_preamble: |_: &mut Self| {},
    }
}

impl<UiState> View<UiState> for SpacerView {
    fn preferred_size(&mut self) -> RectSize {
        self.size
    }

    fn apply_bounds(&mut self, _bounds: Bounds) {}

    fn prepare_for_drawing(
        &mut self,
        _view_context: &ViewContext<UiState>,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _canvas: &crate::wgpu_utils::CanvasView,
    ) {
    }

    fn draw(&self, _view_context: &ViewContext<UiState>, _render_pass: &mut wgpu::RenderPass) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpreadAxis {
    Horizontal,
    Vertical,
}

/// Makes the view take as much space as possible in one axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRef, AsMut, Deref, DerefMut)]
pub struct SpreadView<Subview> {
    axis: SpreadAxis,
    #[as_ref]
    #[as_mut]
    #[deref]
    #[deref_mut]
    subview: Subview,
}

impl<UiState, Subview> View<UiState> for SpreadView<Subview>
where
    Subview: View<UiState>,
{
    fn preferred_size(&mut self) -> RectSize {
        let subview_size = self.subview.preferred_size();
        match self.axis {
            SpreadAxis::Horizontal => RectSize::new(f32::INFINITY, subview_size.height),
            SpreadAxis::Vertical => RectSize::new(subview_size.width, f32::INFINITY),
        }
    }

    fn apply_bounds(&mut self, bounds: Bounds) {
        self.subview.apply_bounds(bounds)
    }

    fn prepare_for_drawing(
        &mut self,
        view_context: &ViewContext<UiState>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        canvas: &crate::wgpu_utils::CanvasView,
    ) {
        self.subview
            .prepare_for_drawing(view_context, device, queue, canvas)
    }

    fn draw(&self, view_context: &ViewContext<UiState>, render_pass: &mut wgpu::RenderPass) {
        self.subview.draw(view_context, render_pass)
    }
}

impl<Subview> SpreadView<Subview> {
    pub fn new(direction: SpreadAxis, subview: Subview) -> Self {
        Self {
            axis: direction,
            subview,
        }
    }

    pub fn horizontal(subview: Subview) -> Self {
        Self::new(SpreadAxis::Horizontal, subview)
    }

    pub fn vertical(subview: Subview) -> Self {
        Self::new(SpreadAxis::Vertical, subview)
    }

    param_getters_setters! {
        vis: pub,
        param_ty: SpreadAxis,
        param: axis,
        param_mut: axis_mut,
        set_param: set_axis,
        with_param: with_axis,
        param_mut_preamble: |_: &mut Self| {},
    }

    pub fn subview(&self) -> &Subview {
        &self.subview
    }

    pub fn subview_mut(&mut self) -> &mut Subview {
        &mut self.subview
    }
}
