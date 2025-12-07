use cgmath::*;

use crate::{
    element::{Bounds, LineWidth, RectElement, RectSize},
    param_getters_setters,
    utils::*,
    view::View,
    wgpu_utils::{CanvasView, Rgba},
};

use super::ViewContext;

#[derive(Debug)]
pub struct RectView {
    size: RectSize,
    fill_color: Rgba,
    line_color: Rgba,
    line_width: LineWidth,
    bounds: Bounds,
    needs_update: bool,
    /// Initialised until the first call of `View::set_size`.
    raw: Option<RectElement>,
}

impl Default for RectView {
    fn default() -> Self {
        Self {
            size: the_default(),
            fill_color: Rgba::from_hex(0xFFFFFF),
            line_color: the_default(),
            line_width: the_default(),
            bounds: the_default(),
            needs_update: true,
            raw: the_default(),
        }
    }
}

impl RectView {
    pub const fn new(size: RectSize) -> Self {
        Self {
            fill_color: Rgba::from_hex(0xFFFFFF),
            line_color: Rgba::from_hex(0xFFFFFF),
            line_width: LineWidth::Uniform(0.),
            size,
            bounds: Bounds::new(point2(0., 0.), size),
            needs_update: true,
            raw: None,
        }
    }

    param_getters_setters! {
        vis: pub,
        param_ty: Rgba,
        param: fill_color,
        param_mut: fill_color_mut,
        set_param: set_fill_color,
        with_param: with_fill_color,
        param_mut_preamble: |self_: &mut Self| self_.needs_update = true,
    }

    param_getters_setters! {
        vis: pub,
        param_ty: Rgba,
        param: line_color,
        param_mut: line_color_mut,
        set_param: set_line_color,
        with_param: with_line_color,
        param_mut_preamble: |self_: &mut Self| self_.needs_update = true,
    }

    param_getters_setters! {
        vis: pub,
        param_ty: LineWidth,
        param: line_width,
        param_mut: line_width_mut,
        set_param: set_line_width,
        with_param: with_line_width,
        param_mut_preamble: |self_: &mut Self| self_.needs_update = true,
    }

    param_getters_setters! {
        vis: pub,
        param_ty: RectSize,
        param: size,
        param_mut: size_mut,
        set_param: set_size,
        with_param: with_size,
        param_mut_preamble: |self_: &mut Self| self_.needs_update = true,
    }

    pub fn bounds(&self) -> Bounds {
        self.bounds
    }

    pub fn set_bounds_(&mut self, bounds: Bounds) {
        self.bounds = bounds;
        self.needs_update = true;
    }
}

impl<UiState> View<UiState> for RectView {
    fn preferred_size(&self) -> RectSize {
        self.size
    }

    fn apply_bounds(&mut self, bounds: Bounds) {
        self.set_bounds_(bounds);
    }

    fn prepare_for_drawing(
        &mut self,
        view_context: &ViewContext<UiState>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        canvas: &CanvasView,
    ) {
        let raw = self
            .raw
            .get_or_insert_with(|| view_context.rect_renderer().create_rect(device));
        // Projection always needs to be set, since `needs_update` does not keep track of canvas
        // size.
        raw.set_projection(queue, canvas.projection);
        if self.needs_update {
            self.needs_update = false;
            raw.set_parameters(queue, self.bounds, self.line_width);
            raw.set_fill_color(queue, self.fill_color);
            raw.set_line_color(queue, self.line_color);
        }
    }

    fn draw(&self, view_context: &ViewContext<UiState>, render_pass: &mut wgpu::RenderPass) {
        if let Some(raw) = self.raw.as_ref()
            && !self.needs_update
        {
            view_context.rect_renderer().draw_rect(render_pass, raw);
        } else {
            log::warn!("`<RectView as View>::draw` is called without `prepare_for_drawing`");
        }
    }
}
