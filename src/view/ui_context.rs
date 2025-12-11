use std::{
    fmt::{self, Display},
    mem::{ManuallyDrop, MaybeUninit},
    ptr::drop_in_place,
    sync::Arc,
};

use cgmath::Point2;
use derive_more::{Display, Error};
use pollster::FutureExt as _;
use winit::window::Window;

use crate::{
    Bounds, Canvas as _, CanvasFormat, CanvasRef, EventRouter, Font, ImageRef, RectSize, Rgba,
    Texture2d, WindowCanvas,
    element::{ImageRenderer, InstancedRectRenderer, RectRenderer, TextRenderer},
    resources::{AppResources, LoadResourceError},
    utils::*,
    view::View,
};

fn init_wgpu() -> (wgpu::Instance, wgpu::Adapter, wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(&the_default());
    let adapter = instance.request_adapter(&the_default()).block_on().unwrap();
    let features = wgpu::FeaturesWGPU::POLYGON_MODE_LINE;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: features.into(),
            ..the_default()
        })
        .block_on()
        .unwrap();
    (instance, adapter, device, queue)
}

/// `'cx` is for allowing `UiState` to contain captured lifetimes, which is necessary for
/// `MouseEventRouter` as it needs to type erase all event listeners.
#[derive(Clone)]
pub struct UiContext<'cx, UiState> {
    device: wgpu::Device,
    queue: wgpu::Queue,
    rect_renderer: RectRenderer<'cx>,
    instanced_rect_renderer: InstancedRectRenderer<'cx>,
    text_renderer: TextRenderer<'cx>,
    image_renderer: ImageRenderer<'cx>,
    event_router: Arc<EventRouter<'cx, UiState>>,
}

impl<'cx, UiState> UiContext<'cx, UiState> {
    pub fn create_for_window(
        resources: &'cx AppResources,
        window: Arc<Window>,
        event_router: Arc<EventRouter<'cx, UiState>>,
    ) -> Result<(Self, WindowCanvas<'static>), UiContextCreationError> {
        let (instance, adapter, device, queue) = init_wgpu();
        let window_canvas =
            WindowCanvas::create_for_window(&instance, &adapter, &device, window.clone());
        let ui_context = UiContext::create(
            device,
            queue,
            resources,
            window_canvas.format(),
            event_router,
        )?;
        Ok((ui_context, window_canvas))
    }

    pub fn create(
        device: wgpu::Device,
        queue: wgpu::Queue,
        resources: &'cx AppResources,
        canvas_format: CanvasFormat,
        event_router: Arc<EventRouter<'cx, UiState>>,
    ) -> Result<Self, UiContextCreationError> {
        macro_rules! try_ {
            ($stage:expr, $x:expr $(,)?) => {
                $x.map_err(|e| UiContextCreationError::new($stage, e))?
            };
        }
        // TODO: Move fonts loading to per-TextElement instance.
        let font = try_!(
            UiContextCreationStage::FontLoading,
            Font::load_from_resources(resources, "fonts/big_blue_terminal.json"),
        );
        let text_renderer = try_!(
            UiContextCreationStage::TextRendererCreation,
            TextRenderer::create(&device, &queue, font, resources, canvas_format),
        );
        let rect_renderer = try_!(
            UiContextCreationStage::RectRendererCreation,
            RectRenderer::create(&device, resources, canvas_format)
        );
        let instanced_rect_renderer = try_!(
            UiContextCreationStage::InstancedRectRendererCreation,
            InstancedRectRenderer::create(&device, resources, canvas_format),
        );
        let image_renderer = try_!(
            UiContextCreationStage::ImageRendererCreation,
            ImageRenderer::create(&device, resources, canvas_format),
        );
        Ok(Self {
            device,
            queue,
            rect_renderer,
            instanced_rect_renderer,
            text_renderer,
            image_renderer,
            event_router,
        })
    }
}

#[derive(Debug, Error)]
pub struct UiContextCreationError {
    stage: UiContextCreationStage,
    error: LoadResourceError,
}

impl UiContextCreationError {
    fn new(stage: UiContextCreationStage, error: LoadResourceError) -> Self {
        Self { stage, error }
    }

    pub fn stage(&self) -> UiContextCreationStage {
        self.stage
    }

    pub fn error(&self) -> &LoadResourceError {
        &self.error
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Error)]
#[non_exhaustive]
pub enum UiContextCreationStage {
    #[display("creating the rect renderer")]
    RectRendererCreation,
    #[display("creating the instanced rect renderer")]
    InstancedRectRendererCreation,
    #[display("loading the font")]
    FontLoading,
    #[display("creating the text renderer")]
    TextRendererCreation,
    #[display("creating the image renderer")]
    ImageRendererCreation,
}

impl Display for UiContextCreationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "when {}, error: {}", self.stage, self.error)
    }
}

impl<'cx, UiState> UiContext<'cx, UiState> {
    pub fn wgpu_device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn wgpu_queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn rect_renderer(&self) -> &RectRenderer<'cx> {
        &self.rect_renderer
    }

    pub fn instanced_rect_renderer(&self) -> &InstancedRectRenderer<'cx> {
        &self.instanced_rect_renderer
    }

    pub fn text_renderer(&self) -> &TextRenderer<'cx> {
        &self.text_renderer
    }

    pub fn image_renderer(&self) -> &ImageRenderer<'cx> {
        &self.image_renderer
    }

    pub fn event_router(&self) -> &Arc<EventRouter<'cx, UiState>> {
        &self.event_router
    }

    pub fn prepare_view(
        &self,
        canvas: &CanvasRef,
        origin: Point2<f32>,
        view: &mut dyn View<'cx, UiState>,
    ) -> Bounds<f32>
    where
        UiState: 'cx,
    {
        let requested_size = view.preferred_size();
        let canvas_size = canvas.logical_size;
        let availible_size = RectSize {
            width: canvas_size.width - origin.x,
            height: canvas_size.height,
        };
        let subview_size = availible_size.min(requested_size);
        let bounds = Bounds::new(origin, subview_size);
        view.apply_bounds(bounds);
        view.prepare_for_drawing(self, canvas);
        bounds
    }

    pub fn prepare_view_bounded(
        &self,
        canvas: &CanvasRef,
        bounds: Bounds<f32>,
        view: &mut dyn View<'cx, UiState>,
    ) where
        UiState: 'cx,
    {
        view.preferred_size();
        view.apply_bounds(bounds);
        view.prepare_for_drawing(self, canvas);
    }

    pub fn draw_view(&self, render_pass: &mut RenderPass, view: &dyn View<'cx, UiState>)
    where
        UiState: 'cx,
    {
        view.draw(self, render_pass);
    }

    pub fn create_texture(&self, image: ImageRef) -> Texture2d {
        Texture2d::create(&self.device, &self.queue, image)
    }

    pub fn begin_render_pass(
        &self,
        canvas: &CanvasRef,
        clear_color: impl Into<Rgba>,
    ) -> RenderPass {
        assert!(
            canvas.depth_stencil_texture_view.is_none(),
            "TODO: drawing with depth stencil buffer"
        );
        let clear_color = clear_color.into();
        let wgpu_clear_color = wgpu::Color {
            r: clear_color.r as f64,
            g: clear_color.g as f64,
            b: clear_color.b as f64,
            a: clear_color.a as f64,
        };
        let mut encoder = self.device.create_command_encoder(&the_default());
        let render_pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &canvas.color_texture_view,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu_clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                    resolve_target: None,
                })],
                ..the_default()
            })
            .forget_lifetime();
        RenderPass::from_raw_parts(self.queue.clone(), render_pass, encoder)
    }
}

pub struct RenderPass {
    queue: wgpu::Queue,
    render_pass: ManuallyDrop<wgpu::RenderPass<'static>>,
    encoder: MaybeUninit<wgpu::CommandEncoder>,
}

unsafe impl Send for RenderPass {}
unsafe impl Sync for RenderPass {}

impl RenderPass {
    pub fn from_raw_parts(
        queue: wgpu::Queue,
        render_pass: wgpu::RenderPass,
        encoder: wgpu::CommandEncoder,
    ) -> Self {
        Self {
            queue,
            render_pass: ManuallyDrop::new(render_pass.forget_lifetime()),
            encoder: MaybeUninit::new(encoder),
        }
    }

    pub fn wgpu_render_pass(&mut self) -> &mut wgpu::RenderPass<'static> {
        &mut self.render_pass
    }
}

impl Drop for RenderPass {
    fn drop(&mut self) {
        unsafe { drop_in_place::<wgpu::RenderPass>(&mut *self.render_pass) };
        let encoder = {
            let mut encoder: MaybeUninit<wgpu::CommandEncoder> = MaybeUninit::uninit();
            std::mem::swap(&mut self.encoder, &mut encoder);
            unsafe { encoder.assume_init() }
        };
        self.queue.submit([encoder.finish()]);
    }
}
