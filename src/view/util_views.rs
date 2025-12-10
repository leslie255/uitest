use derive_more::{AsMut, AsRef, Deref, DerefMut};

use crate::{
    element::{Bounds, RectSize},
    param_getters_setters,
    view::View,
    wgpu_utils::CanvasView,
};

use super::UiContext;

/// An empty view for just leaving a bit of space empty.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct SpacerView {
    size: RectSize<f32>,
}

impl SpacerView {
    pub const fn new(size: RectSize<f32>) -> Self {
        Self { size }
    }

    param_getters_setters! {
        vis: pub,
        param_ty: RectSize<f32>,
        param: size,
        param_mut: size_mut,
        set_param: set_size,
        with_param: with_size,
        param_mut_preamble: |_: &mut Self| {},
    }
}

impl<UiState> View<'_, UiState> for SpacerView {
    fn preferred_size(&mut self) -> RectSize<f32> {
        self.size
    }

    fn apply_bounds(&mut self, _bounds: Bounds<f32>) {}

    fn prepare_for_drawing(
        &mut self,
        _view_context: &UiContext<UiState>,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _canvas: &CanvasView,
    ) {
    }

    fn draw(&self, _view_context: &UiContext<UiState>, _render_pass: &mut wgpu::RenderPass) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpreadAxis {
    Horizontal,
    Vertical,
    Both,
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

impl<'cx, UiState, Subview> View<'cx, UiState> for SpreadView<Subview>
where
    Subview: View<'cx, UiState>,
{
    fn preferred_size(&mut self) -> RectSize<f32> {
        log::debug!("[SpreadView::preferred_size]");
        let subview_size = self.subview.preferred_size();
        match self.axis {
            SpreadAxis::Horizontal => RectSize::new(f32::INFINITY, subview_size.height),
            SpreadAxis::Vertical => RectSize::new(subview_size.width, f32::INFINITY),
            SpreadAxis::Both => RectSize::new(f32::INFINITY, f32::INFINITY),
        }
    }

    fn apply_bounds(&mut self, bounds: Bounds<f32>) {
        self.subview.apply_bounds(bounds)
    }

    fn prepare_for_drawing(
        &mut self,
        ui_context: &UiContext<'cx, UiState>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        canvas: &CanvasView,
    ) {
        self.subview
            .prepare_for_drawing(ui_context, device, queue, canvas)
    }

    fn draw(&self, ui_context: &UiContext<'cx, UiState>, render_pass: &mut wgpu::RenderPass) {
        self.subview.draw(ui_context, render_pass)
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

    pub fn both(subview: Subview) -> Self {
        Self::new(SpreadAxis::Both, subview)
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

    pub fn into_subview(self) -> Subview {
        self.subview
    }

    pub const fn subview(&self) -> &Subview {
        &self.subview
    }

    pub const fn subview_mut(&mut self) -> &mut Subview {
        &mut self.subview
    }
}

/// View that centers a subview within its bounds.
#[derive(Debug, Clone, Copy, PartialEq, AsRef, AsMut, Deref, DerefMut)]
pub struct CenteredView<Subview> {
    size: RectSize<f32>,
    #[as_ref]
    #[as_mut]
    #[deref]
    #[deref_mut]
    subview: Subview,
    subview_size: Option<RectSize<f32>>,
}

impl<Subview> CenteredView<Subview> {
    pub const fn new(size: RectSize<f32>, subview: Subview) -> Self {
        Self {
            size,
            subview,
            subview_size: None,
        }
    }

    param_getters_setters! {
        vis: pub,
        param_ty: RectSize<f32>,
        param: size,
        param_mut: size_mut,
        set_param: set_size,
        with_param: with_size,
        param_mut_preamble: |_: &mut Self| {},
    }

    pub fn into_subview(self) -> Subview {
        self.subview
    }

    pub const fn subview(&self) -> &Subview {
        &self.subview
    }

    pub const fn subview_mut(&mut self) -> &mut Subview {
        &mut self.subview
    }
}

impl<'cx, UiState, Subview> View<'cx, UiState> for CenteredView<Subview>
where
    Subview: View<'cx, UiState>,
{
    fn preferred_size(&mut self) -> RectSize<f32> {
        let subview_size = self.subview.preferred_size();
        self.subview_size = Some(subview_size);
        self.size
    }

    fn apply_bounds(&mut self, bounds: Bounds<f32>) {
        match self.subview_size {
            Some(subview_size) => {
                let padding_left = 0.5 * (bounds.width() - subview_size.width).max(0.);
                let padding_top = 0.5 * (bounds.height() - subview_size.height).max(0.);
                self.subview.apply_bounds(Bounds::from_scalars(
                    bounds.x_min() + padding_left,
                    bounds.y_min() + padding_top,
                    bounds.width() - 2. * padding_left,
                    bounds.height() - 2. * padding_top,
                ))
            }
            None => log::warn!(
                "CenteredView::apply_bounds called without prior CenteredView::preferred_size"
            ),
        }
    }

    fn prepare_for_drawing(
        &mut self,
        ui_context: &UiContext<'cx, UiState>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        canvas: &CanvasView,
    ) {
        self.subview
            .prepare_for_drawing(ui_context, device, queue, canvas)
    }

    fn draw(&self, ui_context: &UiContext<'cx, UiState>, render_pass: &mut wgpu::RenderPass) {
        self.subview.draw(ui_context, render_pass);
    }
}
