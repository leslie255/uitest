use derive_more::{AsMut, AsRef, Deref, DerefMut};

use crate::{
    computed_property,
    element::{Bounds, RectSize},
    property,
    view::{RectView, UiContext, View},
    wgpu_utils::{CanvasView, Rgba},
};

/// An empty view for just leaving a bit of space empty.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct SpacerView {
    size: RectSize<f32>,
}

impl SpacerView {
    pub const fn new(size: RectSize<f32>) -> Self {
        Self { size }
    }

    property! {
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

pub trait ViewExt<'cx, UiState: 'cx>: View<'cx, UiState> + Sized {
    fn into_box_dyn_view(self) -> Box<dyn View<'cx, UiState>> {
        Box::new(self)
    }

    fn into_spread_view(self, axis: SpreadAxis) -> SpreadView<Self> {
        SpreadView::new(axis, self)
    }

    fn into_padded_view(self) -> PaddedView<Self> {
        PaddedView::new(self)
    }

    fn into_ratio_padded_view(self) -> RatioPaddedView<Self> {
        RatioPaddedView::new(self)
    }
}

impl<'cx, UiState: 'cx, T: View<'cx, UiState>> ViewExt<'cx, UiState> for T {}

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
    pub fn new(axis: SpreadAxis, subview: Subview) -> Self {
        Self { axis, subview }
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

    property! {
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

/// View that applys a fixed padding around a subview.
#[derive(Debug, AsRef, AsMut, Deref, DerefMut)]
pub struct PaddedView<Subview> {
    padding_left: f32,
    padding_right: f32,
    padding_top: f32,
    padding_bottom: f32,
    #[as_ref]
    #[as_mut]
    #[deref]
    #[deref_mut]
    subview: Subview,
    subview_size: Option<RectSize<f32>>,
    background_rect: RectView,
}

impl<Subview> PaddedView<Subview> {
    pub fn new(subview: Subview) -> Self {
        Self {
            padding_left: 0.,
            padding_right: 0.,
            padding_top: 0.,
            padding_bottom: 0.,
            subview,
            subview_size: None,
            background_rect: RectView::new(RectSize::new(0., 0.))
                .with_fill_color(Rgba::new(0., 0., 0., 0.)),
        }
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

    property! {
        vis: pub,
        param_ty: f32,
        param: padding_left,
        param_mut: padding_left_mut,
        set_param: set_padding_left,
        with_param: with_padding_left,
        param_mut_preamble: |_: &mut Self| {},
    }

    property! {
        vis: pub,
        param_ty: f32,
        param: padding_right,
        param_mut: padding_right_mut,
        set_param: set_padding_right,
        with_param: with_padding_right,
        param_mut_preamble: |_: &mut Self| {},
    }

    property! {
        vis: pub,
        param_ty: f32,
        param: padding_top,
        param_mut: padding_top_mut,
        set_param: set_padding_top,
        with_param: with_padding_top,
        param_mut_preamble: |_: &mut Self| {},
    }

    property! {
        vis: pub,
        param_ty: f32,
        param: padding_bottom,
        param_mut: padding_bottom_mut,
        set_param: set_padding_bottom,
        with_param: with_padding_bottom,
        param_mut_preamble: |_: &mut Self| {},
    }

    computed_property! {
        vis: pub,
        param_ty: f32,
        param: padding,
        set_param: set_padding,
        with_param: with_padding,
        fset: |self_: &mut Self, padding: f32| {
            self_.set_padding_left(padding);
            self_.set_padding_right(padding);
            self_.set_padding_top(padding);
            self_.set_padding_bottom(padding);
        }
    }

    computed_property! {
        vis: pub,
        param_ty: Rgba,
        param: background_color,
        set_param: set_background_color,
        with_param: with_background_color,
        fset: |self_: &mut Self, background_color: Rgba| {
            self_.background_rect.set_fill_color(background_color);
        }
    }
}

impl<'cx, UiState, Subview> View<'cx, UiState> for PaddedView<Subview>
where
    Subview: View<'cx, UiState>,
{
    fn preferred_size(&mut self) -> RectSize<f32> {
        let subview_size = self.subview.preferred_size();
        self.subview_size = Some(subview_size);
        RectSize {
            width: self.padding_left + subview_size.width + self.padding_right,
            height: self.padding_top + subview_size.height + self.padding_bottom,
        }
    }

    fn apply_bounds(&mut self, bounds: Bounds<f32>) {
        self.background_rect.apply_bounds_(bounds);
        let subview_size = self
            .subview_size
            .unwrap_or_else(|| {
                log::warn!(
                    "PaddedView::apply_bounds called without a prior PaddedView::preferred_size"
                );
                self.subview.preferred_size()
            })
            .min(bounds.size);
        let subview_bounds = Bounds::from_scalars(
            self.padding_left,
            self.padding_top,
            subview_size
                .width
                .min(bounds.width() - self.padding_left - self.padding_right)
                .max(0.),
            subview_size
                .height
                .min(bounds.height() - self.padding_top - self.padding_bottom)
                .max(0.),
        );
        self.subview.apply_bounds(subview_bounds);
    }

    fn prepare_for_drawing(
        &mut self,
        ui_context: &UiContext<'cx, UiState>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        canvas: &CanvasView,
    ) {
        if self.background_rect.fill_color().a != 0. {
            self.background_rect
                .prepare_for_drawing(ui_context, device, queue, canvas);
        }
        self.subview
            .prepare_for_drawing(ui_context, device, queue, canvas);
    }

    fn draw(&self, ui_context: &UiContext<'cx, UiState>, render_pass: &mut wgpu::RenderPass) {
        if self.background_rect.fill_color().a != 0. {
            self.background_rect.draw(ui_context, render_pass);
        }
        self.subview.draw(ui_context, render_pass);
    }
}

/// View that positions a subview within its bounds according to two ratio values:
///
/// - `ratio_top`: value of `padding_top / (padding_top + padding_bottom)`
/// - `ratio_left`: value of `padding_left / (padding_left + padding_right)`
#[derive(Debug, AsRef, AsMut, Deref, DerefMut)]
pub struct RatioPaddedView<Subview> {
    size: Option<RectSize<f32>>,
    ratio_left: f32,
    ratio_top: f32,
    #[as_ref]
    #[as_mut]
    #[deref]
    #[deref_mut]
    subview: Subview,
    subview_size: Option<RectSize<f32>>,
    background_rect: RectView,
}

impl<Subview> RatioPaddedView<Subview> {
    pub fn new(subview: Subview) -> Self {
        Self {
            size: None,
            ratio_left: 0.5,
            ratio_top: 0.5,
            subview,
            subview_size: None,
            background_rect: RectView::new(RectSize::new(0., 0.))
                .with_fill_color(Rgba::new(0., 0., 0., 0.)),
        }
    }

    /// Take as much space as possible.
    pub fn spread(subview: Subview) -> Self {
        Self::new(subview).with_size(RectSize::new(f32::INFINITY, f32::INFINITY))
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

    property! {
        vis: pub,
        param_ty: Option<RectSize<f32>>,
        param: size,
        param_mut: size_mut,
        set_param: set_size,
        with_param: with_size,
        param_mut_preamble: |_: &mut Self| {},
    }

    property! {
        vis: pub,
        param_ty: f32,
        param: ratio_left,
        param_mut: ratio_left_mut,
        set_param: set_ratio_left,
        with_param: with_ratio_left,
        param_mut_preamble: |_: &mut Self| {},
    }

    property! {
        vis: pub,
        param_ty: f32,
        param: ratio_top,
        param_mut: ratio_top_mut,
        set_param: set_ratio_top,
        with_param: with_ratio_top,
        param_mut_preamble: |_: &mut Self| {},
    }

    computed_property! {
        vis: pub,
        param_ty: f32,
        param: ratio_right,
        set_param: set_ratio_right,
        with_param: with_ratio_right,
        fget: |self_: &Self| 1. - self_.ratio_left,
        fset: |self_: &mut Self, ratio_right| self_.ratio_left = 1. - ratio_right,
    }

    computed_property! {
        vis: pub,
        param_ty: f32,
        param: ratio_bottom,
        set_param: set_ratio_bottom,
        with_param: with_ratio_bottom,
        fget: |self_: &Self| 1. - self_.ratio_top,
        fset: |self_: &mut Self, ratio_bottom| self_.ratio_top = 1. - ratio_bottom,
    }

    computed_property! {
        vis: pub,
        param_ty: Rgba,
        param: background_color,
        set_param: set_background_color,
        with_param: with_background_color,
        fget: |self_: &Self| self_.background_rect.fill_color(),
        fset: |self_: &mut Self, background_color| self_.background_rect.set_fill_color(background_color),
    }
}

impl<'cx, UiState, Subview> View<'cx, UiState> for RatioPaddedView<Subview>
where
    Subview: View<'cx, UiState>,
{
    fn preferred_size(&mut self) -> RectSize<f32> {
        let subview_size = self.subview.preferred_size();
        self.subview_size = Some(subview_size);
        self.size.unwrap_or(subview_size)
    }

    fn apply_bounds(&mut self, bounds: Bounds<f32>) {
        self.background_rect.apply_bounds_(bounds);
        match self.subview_size {
            Some(subview_size) => {
                let padding_left = self.ratio_left * (bounds.width() - subview_size.width).max(0.);
                let padding_top = self.ratio_top * (bounds.height() - subview_size.height).max(0.);
                let padding_right =
                    (1. - self.ratio_left) * (bounds.width() - subview_size.width).max(0.);
                let padding_bottom =
                    (1. - self.ratio_top) * (bounds.height() - subview_size.height).max(0.);
                self.subview.apply_bounds(Bounds::from_scalars(
                    bounds.x_min() + padding_left,
                    bounds.y_min() + padding_top,
                    bounds.width() - padding_right,
                    bounds.height() - padding_bottom,
                ))
            }
            None => log::warn!(
                "RatioContainerView::apply_bounds called without prior RatioContainerView::preferred_size"
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
        if self.background_rect.fill_color().a != 0. {
            self.background_rect
                .prepare_for_drawing(ui_context, device, queue, canvas);
        }
        self.subview
            .prepare_for_drawing(ui_context, device, queue, canvas)
    }

    fn draw(&self, ui_context: &UiContext<'cx, UiState>, render_pass: &mut wgpu::RenderPass) {
        if self.background_rect.fill_color().a != 0. {
            self.background_rect.draw(ui_context, render_pass);
        }
        self.subview.draw(ui_context, render_pass);
    }
}
