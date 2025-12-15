use crate::{Bounds, CanvasRef, RectSize, RenderPass, UiContext, View};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContainerPadding {
    /// Fixed padding.
    Fixed(f32),
    /// As ratio of the view's size on that axis.
    ///
    /// FIXME: RatioOfViewSize does not squeeze properly when not enough space is provided.
    RatioOfViewSize(f32),
    /// Take the rest of the remaining space.
    ///
    /// If both edges of an axis are `Spread`, then the view is positioned somewhere in the center.
    /// The position of the view in this situation is determined by `spread_ratio_{vertical|horizontal}`, as follows:
    ///
    /// - `padding_left = spread_ratio_horizontal * (availible_width - subview_width)`
    /// - `padding_top = spread_ratio_vertical * (availible_height - subview_height)`
    /// - `padding_right = (1.0 - spread_ratio_horizontal) * (availible_width - subview_width)`
    /// - `padding_bottom = (1.0 - spread_ratio_vertical) * (availible_height - subview_height)`
    ///
    /// This means that, if paddings on both edges of axis are `Spread`, and that spread ratio of
    /// that axis is `0.5`, then the view is centered on that axis.
    Spread,
}

impl ContainerPadding {
    fn as_fixed(&self) -> Option<f32> {
        if let &Self::Fixed(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl Default for ContainerPadding {
    fn default() -> Self {
        Self::Fixed(0.)
    }
}

pub struct Container<'view, Subview> {
    padding_left: ContainerPadding,
    padding_right: ContainerPadding,
    padding_top: ContainerPadding,
    padding_bottom: ContainerPadding,
    spread_ratio_horizontal: f32,
    spread_ratio_vertical: f32,
    subview: &'view mut Subview,
    subview_size: RectSize<f32>,
    override_size: Option<RectSize<f32>>,
}

impl<'view, Subview> Container<'view, Subview> {
    pub(crate) fn new<'cx, UiState>(subview: &'view mut Subview) -> Self
    where
        UiState: 'cx,
        Subview: View<'cx, UiState>,
    {
        let subview_size = subview.preferred_size();
        Self {
            padding_left: ContainerPadding::Fixed(0.),
            padding_right: ContainerPadding::Fixed(0.),
            padding_top: ContainerPadding::Fixed(0.),
            padding_bottom: ContainerPadding::Fixed(0.),
            spread_ratio_horizontal: 0.5,
            spread_ratio_vertical: 0.5,
            subview,
            subview_size,
            override_size: None,
        }
    }

    pub fn set_padding_left(&mut self, padding_left: impl Into<ContainerPadding>) -> &mut Self {
        self.padding_left = padding_left.into();
        self
    }

    pub fn set_padding_right(&mut self, padding_right: impl Into<ContainerPadding>) -> &mut Self {
        self.padding_right = padding_right.into();
        self
    }

    pub fn set_padding_top(&mut self, padding_top: impl Into<ContainerPadding>) -> &mut Self {
        self.padding_top = padding_top.into();
        self
    }

    pub fn set_padding_bottom(&mut self, padding_bottom: impl Into<ContainerPadding>) -> &mut Self {
        self.padding_bottom = padding_bottom.into();
        self
    }

    pub fn set_padding(&mut self, padding: impl Into<ContainerPadding>) -> &mut Self {
        let padding = padding.into();
        self.set_padding_left(padding);
        self.set_padding_right(padding);
        self.set_padding_top(padding);
        self.set_padding_bottom(padding);
        self
    }

    pub fn set_spread_ratio_horizontal(&mut self, spread_ratio_horizontal: f32) -> &mut Self {
        self.spread_ratio_horizontal = spread_ratio_horizontal;
        self
    }

    pub fn set_spread_ratio_vertical(&mut self, spread_ratio_vertical: f32) -> &mut Self {
        self.spread_ratio_vertical = spread_ratio_vertical;
        self
    }

    /// The preferred size of the subview.
    ///
    /// Does not include the the override size set by `override_subview_size` (if any).
    pub fn subview_size(&self) -> RectSize<f32> {
        self.subview_size
    }

    /// Override the subview size.
    pub fn override_subview_size(
        &mut self,
        override_size: impl Into<Option<RectSize<f32>>>,
    ) -> &mut Self {
        self.override_size = override_size.into();
        self
    }

    fn padding(
        padding_leading: ContainerPadding,
        padding_trailing: ContainerPadding,
        spread_ratio: f32,
        view_length: f32,
        remaining_length: f32,
    ) -> (f32, f32) {
        use ContainerPadding::*;
        let padding = |padding: ContainerPadding| match padding {
            Fixed(fixed) => fixed,
            RatioOfViewSize(ratio) => ratio * view_length,
            Spread => spread_ratio * remaining_length,
        };
        match (padding_leading, padding_trailing) {
            (Spread, Spread) => (
                spread_ratio * remaining_length,
                spread_ratio * remaining_length,
            ),
            (leading, Spread) => {
                let padding_leading = padding(leading);
                (padding_leading, (remaining_length - padding_leading))
            }
            (Spread, trailing) => {
                let padding_trailing = padding(trailing);
                ((remaining_length - padding_trailing), padding_trailing)
            }
            (leading, trailing) => (padding(leading), padding(trailing)),
        }
    }
}

impl<'view, 'cx, UiState, Subview> View<'cx, UiState> for Container<'view, Subview>
where
    UiState: 'cx,
    Subview: View<'cx, UiState>,
{
    fn preferred_size(&mut self) -> RectSize<f32> {
        let subview_size = self.override_size.unwrap_or(self.subview_size);
        let (padding_left, padding_right) = Self::padding(
            self.padding_left,
            self.padding_right,
            self.spread_ratio_horizontal,
            subview_size.width,
            f32::INFINITY,
        );
        let (padding_top, padding_bottom) = Self::padding(
            self.padding_top,
            self.padding_bottom,
            self.spread_ratio_vertical,
            subview_size.height,
            f32::INFINITY,
        );
        RectSize {
            width: padding_left + subview_size.width + padding_right,
            height: padding_top + subview_size.height + padding_bottom,
        }
    }

    fn apply_bounds(&mut self, bounds: Bounds<f32>) {
        let requested_size = self.override_size.unwrap_or(self.subview_size);
        let max_size = RectSize {
            width: (bounds.width()
                - self.padding_left.as_fixed().unwrap_or(0.)
                - self.padding_right.as_fixed().unwrap_or(0.)),
            height: (bounds.height()
                - self.padding_top.as_fixed().unwrap_or(0.)
                - self.padding_bottom.as_fixed().unwrap_or(0.)),
        }
        .max(RectSize::new(0., 0.));
        let subview_size = requested_size.min(max_size);
        let (padding_left, padding_right) = Self::padding(
            self.padding_left,
            self.padding_right,
            self.spread_ratio_horizontal,
            subview_size.width,
            (bounds.width() - subview_size.width).max(0.),
        );
        let (padding_top, padding_bottom) = Self::padding(
            self.padding_top,
            self.padding_bottom,
            self.spread_ratio_vertical,
            subview_size.height,
            (bounds.height() - subview_size.height).max(0.),
        );
        let padded_size = RectSize {
            width: padding_left + subview_size.width + padding_right,
            height: padding_top + subview_size.height + padding_bottom,
        };
        let shrink_horizontal = (bounds.width() / padded_size.width).min(1.);
        let shrink_vertical = (bounds.height() / padded_size.height).min(1.);
        let mut subview_bounds = Bounds::from_scalars(
            bounds.x_min() + padding_left,
            bounds.y_min() + padding_top,
            subview_size.width * shrink_horizontal,
            subview_size.height * shrink_vertical,
        );
        if subview_bounds.x_max() > bounds.x_max() {
            subview_bounds.size.width = (bounds.x_max() - subview_bounds.x_min()).max(0.);
        }
        if subview_bounds.y_max() > bounds.y_max() {
            subview_bounds.size.height = (bounds.y_max() - subview_bounds.y_min()).max(0.);
        }
        self.subview.apply_bounds(subview_bounds);
    }

    fn prepare_for_drawing(&mut self, ui_context: &UiContext<'cx, UiState>, canvas: &CanvasRef) {
        self.subview.prepare_for_drawing(ui_context, canvas);
    }

    fn draw(&self, ui_context: &UiContext<'cx, UiState>, render_pass: &mut RenderPass) {
        self.subview.draw(ui_context, render_pass);
    }
}
