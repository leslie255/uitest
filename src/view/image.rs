use crate::{
    element::{Bounds, ImageElement, RectSize, Texture2d},
    property,
    utils::*,
    view::{UiContext, View},
    wgpu_utils::CanvasView,
};

#[derive(Debug, Clone)]
pub struct ImageView {
    size: RectSize<f32>,
    bounds: Bounds<f32>,
    bounds_updated: bool,
    texture: Option<Texture2d>,
    texture_updated: bool,
    raw: Option<ImageElement>,
}

impl ImageView {
    pub fn new(size: RectSize<f32>) -> Self {
        Self {
            size,
            bounds: the_default(),
            bounds_updated: false,
            texture: None,
            texture_updated: false,
            raw: None,
        }
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

    pub fn texture(&self) -> Option<&Texture2d> {
        self.texture.as_ref()
    }

    pub fn texture_mut(&mut self) -> &mut Option<Texture2d> {
        self.texture_updated = true;
        &mut self.texture
    }

    pub fn set_texture(&mut self, texture: impl Into<Option<Texture2d>>) {
        *self.texture_mut() = texture.into();
    }

    pub fn with_texture(mut self, texture: impl Into<Option<Texture2d>>) -> Self {
        *self.texture_mut() = texture.into();
        self
    }

    /// Set the preferred size to size of the texture.
    pub fn resize_to_fit(&mut self) {
        if let Some(texture) = self.texture.as_ref() {
            self.set_size(texture.size());
        }
    }

    pub fn apply_bounds_(&mut self, bounds: Bounds<f32>) {
        self.bounds = bounds;
        self.bounds_updated = true
    }
}

impl<UiState> View<'_, UiState> for ImageView {
    fn preferred_size(&mut self) -> RectSize<f32> {
        self.size
    }

    fn apply_bounds(&mut self, bounds: Bounds<f32>) {
        self.apply_bounds_(bounds);
    }

    fn prepare_for_drawing(
        &mut self,
        ui_context: &UiContext<UiState>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        canvas: &CanvasView,
    ) {
        if (self.texture_updated || self.raw.is_none())
            && let Some(texture) = self.texture.as_ref()
        {
            self.raw = Some(ui_context.image_renderer().create_image(device, texture));
        }
        if let Some(raw) = self.raw.as_ref() {
            raw.set_projection(queue, canvas.projection);
            if self.bounds_updated {
                self.bounds_updated = false;
                raw.set_parameters(queue, self.bounds);
            }
        }
    }

    fn draw(&self, ui_context: &UiContext<UiState>, render_pass: &mut wgpu::RenderPass) {
        if let Some(raw) = self.raw.as_ref() {
            ui_context.image_renderer().draw_image(render_pass, raw);
        }
    }
}
