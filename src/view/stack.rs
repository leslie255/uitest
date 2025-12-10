use std::marker::PhantomData;

use cgmath::*;

use crate::{
    element::{Bounds, RectSize},
    param_getters_setters,
    view::{Axis, Point2Ext as _, RectView, View, ViewList},
    wgpu_utils::{CanvasView, Rgba},
};

use super::{ControlFlow, UiContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackPaddingType {
    /// Pad only between the subviews.
    Interpadded,
    /// Pad between the subviews, before the first subview, and after the last subview.
    Omnipadded,
}

pub struct StackView<'cx, Subviews: ViewList<'cx>> {
    axis: Axis,
    subviews: Subviews,
    background_view: Option<RectView>,
    padding_type: StackPaddingType,
    fixed_padding: Option<f32>,
    subview_sizes: Vec<RectSize<f32>>,
    subview_length_alpha_total: f32,
    _marker: PhantomData<&'cx ()>,
}

impl<'cx, Subviews: ViewList<'cx>> StackView<'cx, Subviews> {
    pub const fn new(axis: Axis, subviews: Subviews) -> Self {
        Self {
            axis,
            subviews,
            background_view: None,
            padding_type: StackPaddingType::Interpadded,
            fixed_padding: None,
            subview_sizes: Vec::new(),
            subview_length_alpha_total: 0.0f32,
            _marker: PhantomData,
        }
    }

    pub const fn horizontal(subviews: Subviews) -> Self {
        Self::new(Axis::Horizontal, subviews)
    }

    pub const fn vertical(subviews: Subviews) -> Self {
        Self::new(Axis::Vertical, subviews)
    }

    param_getters_setters! {
        vis: pub,
        param_ty: Axis,
        param: axis,
        param_mut: axis_mut,
        set_param: set_axis,
        with_param: with_axis,
        param_mut_preamble: |_: &mut Self| (),
    }

    param_getters_setters! {
        vis: pub,
        param_ty: StackPaddingType,
        param: padding_type,
        param_mut: padding_type_mut,
        set_param: set_padding_type,
        with_param: with_padding_type,
        param_mut_preamble: |_: &mut Self| (),
    }

    param_getters_setters! {
        vis: pub,
        param_ty: Option<f32>,
        param: fixed_padding,
        param_mut: fixed_padding_mut,
        set_param: set_fixed_padding,
        with_param: with_fixed_padding,
        param_mut_preamble: |_: &mut Self| (),
    }

    pub fn background_color(&self) -> Rgba {
        self.background_view
            .as_ref()
            .map_or(Rgba::from_hex(0), |background_view| {
                background_view.fill_color()
            })
    }

    pub fn set_background_color(&mut self, background_color: impl Into<Rgba>) {
        let background_color: Rgba = background_color.into();
        if background_color.a == 0. {
            return;
        }
        let background_view = self
            .background_view
            .get_or_insert_with(|| RectView::new(RectSize::new(0., 0.)));
        background_view.set_fill_color(background_color);
    }

    pub fn with_background_color(mut self, background_color: impl Into<Rgba>) -> Self {
        self.set_background_color(background_color);
        self
    }

    fn warn_n_subviews_changed() {
        log::warn!(
            "`StackView::apply_bounds` called, but number of subviews have changed since `StackView::preferred_size`"
        );
    }
}

impl<'cx, Subviews: ViewList<'cx>> View<'cx, Subviews::UiState> for StackView<'cx, Subviews> {
    fn preferred_size(&mut self) -> RectSize<f32> {
        let mut length_alpha = 0.0f32;
        let mut length_beta = 0.0f32;
        self.subview_sizes.clear();
        self.subviews.for_each_subview_mut(|subview| {
            let subview_size = subview.preferred_size();
            self.subview_sizes.push(subview_size);
            length_alpha += subview_size.length_alpha(self.axis);
            length_beta = length_beta.max(subview_size.length_beta(self.axis));
            ControlFlow::Continue
        });
        let n_subviews = self.subview_sizes.len();
        let n_paddings = match self.padding_type() {
            StackPaddingType::Interpadded => n_subviews.saturating_sub(1),
            StackPaddingType::Omnipadded => n_subviews + 1,
        };
        self.subview_length_alpha_total = length_alpha;
        let padding_total = (n_paddings as f32) * self.fixed_padding.unwrap_or(0.);
        length_alpha += padding_total;
        RectSize::new_on_axis(self.axis, length_alpha, length_beta)
    }

    fn apply_bounds(&mut self, bounds: Bounds<f32>) {
        if let Some(background_view) = self.background_view.as_mut() {
            background_view.apply_bounds_(bounds);
        }
        let mut subview_sizes = self.subview_sizes.iter();
        let n_subviews = self.subview_sizes.len();
        let n_paddings = match self.padding_type {
            StackPaddingType::Interpadded => n_subviews.saturating_sub(1),
            StackPaddingType::Omnipadded => n_subviews + 1,
        };
        let padding = match self.fixed_padding {
            Some(fixed_padding) => fixed_padding,
            None => {
                (bounds.length_alpha(self.axis) - self.subview_length_alpha_total)
                    / (n_paddings as f32)
            }
        };
        let mut offset_alpha = match self.padding_type {
            StackPaddingType::Interpadded => bounds.alpha_min(self.axis) + 0.,
            StackPaddingType::Omnipadded => bounds.alpha_min(self.axis) + padding,
        };
        self.subviews.for_each_subview_mut(|subview| {
            let Some(&requested_size) = subview_sizes.next() else {
                Self::warn_n_subviews_changed();
                return ControlFlow::Break;
            };
            let remaining_size = RectSize::new_on_axis(
                self.axis, //
                bounds.length_alpha(self.axis) - offset_alpha + bounds.alpha_min(self.axis),
                bounds.length_beta(self.axis),
            );
            let subview_size = requested_size.min(remaining_size);
            let offset_beta = bounds.beta_min(self.axis)
                + 0.5 * (bounds.length_beta(self.axis) - subview_size.length_beta(self.axis));
            let subview_bounds = Bounds::new(
                Point2::new_on_axis(self.axis, offset_alpha, offset_beta),
                subview_size,
            );
            subview.apply_bounds(subview_bounds);
            offset_alpha += padding;
            offset_alpha += subview_size.length_alpha(self.axis);
            ControlFlow::Continue
        });
    }

    fn prepare_for_drawing(
        &mut self,
        ui_context: &UiContext<'cx, Subviews::UiState>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        canvas: &CanvasView,
    ) {
        if let Some(background_view) = self.background_view.as_mut()
            && background_view.fill_color().a != 0.
        {
            background_view.prepare_for_drawing(ui_context, device, queue, canvas);
        }
        self.subviews.for_each_subview_mut(|subview| {
            subview.prepare_for_drawing(ui_context, device, queue, canvas);
            ControlFlow::Continue
        });
    }

    fn draw(
        &self,
        ui_context: &UiContext<'cx, Subviews::UiState>,
        render_pass: &mut wgpu::RenderPass,
    ) {
        if let Some(background_view) = self.background_view.as_ref()
            && background_view.fill_color().a != 0.
        {
            background_view.draw(ui_context, render_pass);
        }
        self.subviews.for_each_subview(|subview| {
            subview.draw(ui_context, render_pass);
            ControlFlow::Continue
        });
    }
}
