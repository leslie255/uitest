use std::{
    fmt::{self, Display},
    sync::Arc,
};

use cgmath::Point2;
use derive_more::{Display, Error};

use crate::{
    element::{Bounds, Font, InstancedRectRenderer, RectRenderer, TextRenderer},
    mouse_event::MouseEventRouter,
    resources::{AppResources, LoadResourceError},
    view::View,
    wgpu_utils::{CanvasFormat, CanvasView},
};

/// `'cx` is for allowing `UiState` to contain captured lifetimes, which is necessary for
/// `MouseEventRouter` as it needs to type erase all event listeners.
#[derive(Clone)]
pub struct ViewContext<'cx, UiState> {
    rect_renderer: RectRenderer<'cx>,
    instanced_rect_renderer: InstancedRectRenderer<'cx>,
    text_renderer: TextRenderer<'cx>,
    mouse_event_router: Arc<MouseEventRouter<'cx, UiState>>,
}

impl<'cx, UiState> ViewContext<'cx, UiState> {
    pub fn create(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        resources: &'cx AppResources,
        canvas_format: CanvasFormat,
        mouse_event_router: Arc<MouseEventRouter<'cx, UiState>>,
    ) -> Result<Self, ViewContextCreationError> {
        macro_rules! try_ {
            ($stage:expr, $x:expr $(,)?) => {
                $x.map_err(|e| ViewContextCreationError::new($stage, e))?
            };
        }
        // TODO: Move fonts loading to per-TextElement instance.
        let font = try_!(
            ViewContextCreationStage::FontLoading,
            Font::load_from_path(resources, "fonts/big_blue_terminal.json"),
        );
        let text_renderer = try_!(
            ViewContextCreationStage::TextRendererCreation,
            TextRenderer::create(device, queue, font, resources, canvas_format),
        );
        let rect_renderer = try_!(
            ViewContextCreationStage::RectRendererCreation,
            RectRenderer::create(device, resources, canvas_format)
        );
        let instanced_rect_renderer = try_!(
            ViewContextCreationStage::InstancedRectRenderer,
            InstancedRectRenderer::create(device, resources, canvas_format),
        );
        Ok(Self {
            rect_renderer,
            instanced_rect_renderer,
            text_renderer,
            mouse_event_router,
        })
    }
}

#[derive(Debug, Error)]
pub struct ViewContextCreationError {
    stage: ViewContextCreationStage,
    error: LoadResourceError,
}

impl ViewContextCreationError {
    fn new(stage: ViewContextCreationStage, error: LoadResourceError) -> Self {
        Self { stage, error }
    }

    pub fn stage(&self) -> ViewContextCreationStage {
        self.stage
    }

    pub fn error(&self) -> &LoadResourceError {
        &self.error
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Error)]
#[non_exhaustive]
pub enum ViewContextCreationStage {
    #[display("creating the rect renderer")]
    RectRendererCreation,
    #[display("creating the instanced rect renderer")]
    InstancedRectRenderer,
    #[display("loading the font")]
    FontLoading,
    #[display("creating the text renderer")]
    TextRendererCreation,
}

impl Display for ViewContextCreationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "when {}, error: {}", self.stage, self.error)
    }
}

impl<'cx, UiState> ViewContext<'cx, UiState> {
    pub fn rect_renderer(&self) -> &RectRenderer<'cx> {
        &self.rect_renderer
    }

    pub fn instanced_rect_renderer(&self) -> &InstancedRectRenderer<'cx> {
        &self.instanced_rect_renderer
    }

    pub fn text_renderer(&self) -> &TextRenderer<'cx> {
        &self.text_renderer
    }

    pub fn mouse_event_router(&self) -> &Arc<MouseEventRouter<'cx, UiState>> {
        &self.mouse_event_router
    }

    pub fn prepare_view(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        canvas: &CanvasView,
        origin: Point2<f32>,
        view: &mut impl View<UiState>,
    ) -> Bounds {
        let size = view.preferred_size();
        let bounds = Bounds::new(origin, size);
        self.prepare_view_bounded(device, queue, canvas, bounds, view);
        bounds
    }

    pub fn prepare_view_bounded(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        canvas: &CanvasView,
        bounds: Bounds,
        view: &mut impl View<UiState>,
    ) {
        view.apply_bounds(bounds);
        view.prepare_for_drawing(self, device, queue, canvas);
    }

    pub fn draw_view(&self, render_pass: &mut wgpu::RenderPass, view: &impl View<UiState>) {
        view.draw(self, render_pass);
    }
}
