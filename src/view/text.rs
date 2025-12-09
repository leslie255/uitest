use std::cell::OnceCell;

use cgmath::*;

use crate::{
    element::{Bounds, Font, RectSize, TextElement},
    param_getters_setters,
    view::{UiContext, View},
    wgpu_utils::Rgba,
};

#[derive(Debug)]
pub struct TextView<'cx> {
    n_lines: usize,
    n_columns: usize,
    text: String,
    font_size: f32,
    font: Font<'cx>,
    fg_color: Rgba,
    bg_color: Rgba,
    origin: Point2<f32>,
    needs_update: bool,
    text_needs_update: bool,
    raw: OnceCell<TextElement>,
}

impl<'cx> TextView<'cx> {
    pub fn new<UiState>(ui_context: &UiContext<'cx, UiState>) -> Self {
        Self {
            n_lines: 1,
            n_columns: 0,
            text: String::new(),
            font_size: 12.,
            font: ui_context.text_renderer().font(),
            fg_color: Rgba::from_hex(0xFFFFFF),
            bg_color: Rgba::from_hex(0x00000000),
            origin: point2(0., 0.),
            needs_update: false,
            text_needs_update: false,
            raw: OnceCell::new(),
        }
    }

    param_getters_setters! {
        vis: pub,
        param_ty: f32,
        param: font_size,
        param_mut: font_size_mut,
        set_param: set_font_size,
        with_param: with_font_size,
        param_mut_preamble: |self_: &mut Self| self_.needs_update = true,
    }

    param_getters_setters! {
        vis: pub,
        param_ty: Rgba,
        param: fg_color,
        param_mut: fg_color_mut,
        set_param: set_fg_color,
        with_param: with_fg_color,
        param_mut_preamble: |self_: &mut Self| self_.needs_update = true,
    }

    param_getters_setters! {
        vis: pub,
        param_ty: Rgba,
        param: bg_color,
        param_mut: bg_color_mut,
        set_param: set_bg_color,
        with_param: with_bg_color,
        param_mut_preamble: |self_: &mut Self| self_.needs_update = true,
    }

    pub fn set_text(&mut self, text: String) {
        self.text_needs_update = true;
        self.n_lines = 1usize;
        let mut n_columns = 0usize;
        self.n_columns = 0;
        self.n_lines = 1;
        for char in text.chars() {
            match char {
                '\n' => {
                    self.n_lines += 1;
                    self.n_columns = self.n_columns.max(n_columns);
                    n_columns = 0;
                }
                '\r' => {
                    self.n_columns = self.n_columns.max(n_columns);
                    n_columns = 0;
                }
                _ => {
                    n_columns += 1;
                    self.n_columns = self.n_columns.max(n_columns)
                }
            }
        }
        self.text = text;
    }

    pub fn n_columns(&self) -> usize {
        self.n_columns
    }

    pub fn n_lines(&self) -> usize {
        self.n_lines
    }

    pub fn size(&self) -> RectSize<f32> {
        RectSize::new(
            (self.n_columns as f32) * self.font.glyph_relative_width() * self.font_size(),
            self.n_lines as f32 * self.font_size(),
        )
    }

    pub fn set_bounds_(&mut self, bounds: Bounds<f32>) {
        self.needs_update = true;
        self.origin = bounds.origin;
    }
}

impl<'cx, UiState> View<'cx, UiState> for TextView<'cx> {
    fn preferred_size(&mut self) -> RectSize<f32> {
        self.size()
    }

    fn apply_bounds(&mut self, bounds: Bounds<f32>) {
        self.set_bounds_(bounds);
    }

    fn prepare_for_drawing(
        &mut self,
        ui_context: &UiContext<UiState>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        canvas: &crate::wgpu_utils::CanvasView,
    ) {
        let raw = self.raw.get_or_init(|| {
            self.text_needs_update = false; // `create_text` updates the text
            ui_context.text_renderer().create_text(device, &self.text)
        });
        // Projection always needs to be set, since `needs_update` does not keep track of canvas
        // size.
        raw.set_projection(queue, canvas.projection);
        if self.needs_update {
            self.needs_update = false;
            raw.set_parameters(queue, self.origin, self.font_size);
            raw.set_fg_color(queue, self.fg_color);
            raw.set_bg_color(queue, self.bg_color);
        }
        if self.text_needs_update {
            self.text_needs_update = false;
            let raw = self.raw.get_mut().unwrap();
            ui_context
                .text_renderer()
                .update_text(device, raw, &self.text);
        }
    }

    fn draw(&self, ui_context: &UiContext<UiState>, render_pass: &mut wgpu::RenderPass) {
        ui_context
            .text_renderer()
            .draw_text(render_pass, self.raw.get().unwrap());
    }
}
