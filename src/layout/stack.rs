use crate::{
    Axis, Bounds, CanvasRef, RectSize, RenderPass, UiContext, View, axis_utils::*,
};

use bumpalo::{Bump, collections::Vec as BumpVec};
use cgmath::*;
use derive_more::{AsMut, AsRef, Deref, DerefMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackPaddingType {
    /// Pad only between the subviews.
    Interpadded,
    /// Pad between the subviews, before the first subview, and after the last subview.
    Omnipadded,
}

#[derive(Default, Debug, Clone, Copy)]
pub enum StackAlignmentHorizontal {
    #[default]
    Center,
    Left,
    Right,
    Ratio(f32),
}

#[derive(Default, Debug, Clone, Copy)]
pub enum StackAlignmentVertical {
    #[default]
    Center,
    Top,
    Bottom,
    Ratio(f32),
}

/// Axis-generic version over `StackAlignmentHorizontal` and `StackAlignmentVertical`.
#[derive(Default, Debug, Clone, Copy)]
pub(crate) enum StackAlignment {
    #[default]
    Center,
    Leading,
    Trailing,
    Ratio(f32),
}

impl StackAlignment {
    fn ratio(self) -> f32 {
        match self {
            StackAlignment::Center => 0.5,
            StackAlignment::Leading => 0.0,
            StackAlignment::Trailing => 1.0,
            StackAlignment::Ratio(ratio) => ratio,
        }
    }
}

impl From<StackAlignmentHorizontal> for StackAlignment {
    fn from(alignment: StackAlignmentHorizontal) -> Self {
        match alignment {
            StackAlignmentHorizontal::Center => Self::Center,
            StackAlignmentHorizontal::Left => Self::Leading,
            StackAlignmentHorizontal::Right => Self::Trailing,
            StackAlignmentHorizontal::Ratio(r) => Self::Ratio(r),
        }
    }
}

impl From<StackAlignmentVertical> for StackAlignment {
    fn from(alignment: StackAlignmentVertical) -> Self {
        match alignment {
            StackAlignmentVertical::Center => Self::Center,
            StackAlignmentVertical::Top => Self::Leading,
            StackAlignmentVertical::Bottom => Self::Trailing,
            StackAlignmentVertical::Ratio(r) => Self::Ratio(r),
        }
    }
}

impl From<StackAlignment> for StackAlignmentHorizontal {
    fn from(alignment: StackAlignment) -> Self {
        match alignment {
            StackAlignment::Center => Self::Center,
            StackAlignment::Leading => Self::Left,
            StackAlignment::Trailing => Self::Right,
            StackAlignment::Ratio(r) => Self::Ratio(r),
        }
    }
}

impl From<StackAlignment> for StackAlignmentVertical {
    fn from(alignment: StackAlignment) -> Self {
        match alignment {
            StackAlignment::Center => Self::Center,
            StackAlignment::Leading => Self::Top,
            StackAlignment::Trailing => Self::Bottom,
            StackAlignment::Ratio(r) => Self::Ratio(r),
        }
    }
}

#[derive(AsRef, AsMut, Deref, DerefMut)]
pub(crate) struct Subview<'a, 'cx, UiState> {
    pub(crate) preferred_size: RectSize<f32>,
    #[deref]
    #[deref_mut]
    #[as_ref]
    #[as_mut]
    pub(crate) view: &'a mut (dyn View<'cx, UiState> + 'a),
}

pub struct Stack<'pass, 'views, 'cx, UiState> {
    axis: Axis,
    alignment_alpha: StackAlignment,
    alignment_beta: StackAlignment,
    padding_type: StackPaddingType,
    fixed_padding: Option<f32>,
    shrink_together: bool,
    subviews: BumpVec<'pass, Subview<'views, 'cx, UiState>>,
    /// Sum of the alphas of subviews.
    ///
    /// For the lingo "a", "b", "alpha", "beta", see `axis_utils`.
    alpha_sum: f32,
    /// Max among the betas of subviews.
    ///
    /// For the lingo "a", "b", "alpha", "beta", see `axis_utils`.
    beta_max: f32,
    /// Max among the betas of subviews, excluding those who has infinite betas.
    ///
    /// For the lingo "a", "b", "alpha", "beta", see `axis_utils`.
    beta_max_finite: f32,
}

impl<'pass, 'views, 'cx, UiState> Stack<'pass, 'views, 'cx, UiState> {
    pub(crate) fn new(bump: &'pass Bump, axis: Axis) -> Self {
        Self {
            axis,
            alignment_alpha: StackAlignment::Center,
            alignment_beta: StackAlignment::Center,
            padding_type: StackPaddingType::Interpadded,
            fixed_padding: None,
            shrink_together: false,
            subviews: BumpVec::new_in(bump),
            alpha_sum: 0.,
            beta_max: 0.,
            beta_max_finite: 0.,
        }
    }

    pub(crate) fn subview(&mut self, subview: &'views mut (dyn View<'cx, UiState> + 'views)) {
        // For the lingo "a", "b", "alpha", "beta", see `axis_utils`.
        let subview_size = subview.preferred_size();
        self.alpha_sum += subview_size.alpha(self.axis);
        let subview_beta = subview_size.beta(self.axis);
        self.beta_max = self.beta_max.max(subview_beta);
        if subview_beta.is_finite() {
            self.beta_max_finite = self.beta_max_finite.max(subview_beta);
        }
        self.subviews.push(Subview {
            preferred_size: subview_size,
            view: subview,
        });
    }

    fn n_paddings(n_subviews: usize, padding_type: StackPaddingType) -> usize {
        match padding_type {
            StackPaddingType::Interpadded => n_subviews.saturating_sub(1),
            StackPaddingType::Omnipadded => n_subviews + 1,
        }
    }
}

impl<'pass, 'views, 'cx, UiState> View<'cx, UiState> for Stack<'pass, 'views, 'cx, UiState> {
    fn preferred_size(&mut self) -> RectSize<f32> {
        let n_paddings = Self::n_paddings(self.subviews.len(), self.padding_type) as f32;
        RectSize::new_on_axis(
            self.axis,
            self.alpha_sum + n_paddings * self.fixed_padding.unwrap_or(0.),
            self.beta_max,
        )
    }

    fn apply_bounds(&mut self, bounds: Bounds<f32>) {
        // For the lingo "a", "b", "alpha", "beta", see `axis_utils`.

        let n_paddings = Self::n_paddings(self.subviews.len(), self.padding_type) as f32;

        let min_alpha = self.alpha_sum + n_paddings * self.fixed_padding.unwrap_or(0.);
        let leftover_alpha = (bounds.alpha(self.axis) - min_alpha).max(0.);
        let shrink_a = (bounds.alpha(self.axis) / min_alpha).min(1.);
        let shrink_b = match self.shrink_together {
            true => (bounds.beta(self.axis) / self.beta_max_finite).min(1.),
            false => 1.0f32,
        };
        let padding_leading = match (self.fixed_padding, self.alignment_alpha) {
            // Alignment_alpha is only effective if with fixed padding.
            (None, _) => 0.0f32,
            (Some(_), alignment) => alignment.ratio() * leftover_alpha,
        };
        let padding_body = match self.fixed_padding {
            Some(fixed_padding) => fixed_padding * shrink_a,
            None => leftover_alpha / n_paddings,
        };

        // Accumulator for A-axis offset while we iterate through the subviews.
        let mut offset_a = padding_leading;
        for (i, subview) in self.subviews.iter_mut().enumerate() {
            if i != 0 || self.padding_type == StackPaddingType::Omnipadded {
                // This accumulation cannot be moved to end of iteration to eliminate the if
                // condition, because `remaining_size` uses offset_a later.
                offset_a += padding_body;
            }
            let requested_size = subview
                .preferred_size
                .scaled_on_axis(self.axis, shrink_a, shrink_b);
            let remaining_size = RectSize::new_on_axis(
                self.axis,
                bounds.alpha(self.axis) - offset_a,
                bounds.beta(self.axis),
            );
            let subview_size = requested_size.min(remaining_size);
            let leftover_beta = bounds.beta(self.axis) - subview_size.beta(self.axis);
            let offset_b = self.alignment_beta.ratio() * leftover_beta;
            let subview_bounds = Bounds::new(
                bounds.origin + Vector2::new_on_axis(self.axis, offset_a, offset_b),
                subview_size,
            );
            subview.apply_bounds(subview_bounds);
            offset_a += subview_size.alpha(self.axis);
        }
    }

    fn prepare_for_drawing(&mut self, ui_context: &UiContext<'cx, UiState>, canvas: &CanvasRef) {
        for subview in &mut self.subviews {
            subview.prepare_for_drawing(ui_context, canvas);
        }
    }

    fn draw(&self, ui_context: &UiContext<'cx, UiState>, render_pass: &mut RenderPass) {
        for subview in &self.subviews {
            subview.draw(ui_context, render_pass);
        }
    }
}

pub struct StackBuilder<'pass, 'views, 'cx, UiState> {
    stack: Stack<'pass, 'views, 'cx, UiState>,
}

impl<'pass, 'views, 'cx, UiState> StackBuilder<'pass, 'views, 'cx, UiState> {
    pub(crate) fn new(bump: &'pass Bump, axis: Axis) -> Self {
        Self {
            stack: Stack::new(bump, axis),
        }
    }

    pub fn subview(&mut self, subview: &'views mut (dyn View<'cx, UiState> + 'views)) {
        self.stack.subview(subview);
    }

    pub fn set_alignment_vertical(&mut self, alignment: StackAlignmentVertical) {
        match self.stack.axis {
            Axis::Horizontal => self.stack.alignment_beta = alignment.into(),
            Axis::Vertical => self.stack.alignment_alpha = alignment.into(),
        }
    }

    pub fn set_alignment_horizontal(&mut self, alignment: StackAlignmentHorizontal) {
        match self.stack.axis {
            Axis::Horizontal => self.stack.alignment_alpha = alignment.into(),
            Axis::Vertical => self.stack.alignment_beta = alignment.into(),
        }
    }

    /// See `StackPaddingType`.
    ///
    /// Default value: `StackPaddingType::Interpadded`.
    pub fn set_padding_type(&mut self, padding_type: StackPaddingType) {
        self.stack.padding_type = padding_type;
    }

    /// If `Some`, the paddings would be of fixed size.
    ///
    /// If `None`, the paddings would be automatically adjusted such that they divide the empty
    /// space equally.
    ///
    /// Default value: `None`.
    pub fn set_fixed_padding(&mut self, fixed_padding: impl Into<Option<f32>>) {
        self.stack.fixed_padding = fixed_padding.into();
    }

    /// For a vertical stack, if it does not have enough space horizontally, should it shrink all
    /// rows together at the same rate, or independently per-row? Vice-versa for horizontal stacks.
    ///
    /// Default value: `false` (i.e. shrink together).
    pub fn set_shrink_together(&mut self, shrink_together: bool) {
        self.stack.shrink_together = shrink_together;
    }

    pub(crate) fn finish(self) -> Stack<'pass, 'views, 'cx, UiState> {
        self.stack
    }
}
