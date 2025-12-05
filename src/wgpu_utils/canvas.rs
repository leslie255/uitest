use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use cgmath::*;
use derive_more::{Display, Error};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{shapes::RectSize, utils::*};

pub trait Canvas {
    fn format(&self) -> CanvasFormat;
    fn logical_size(&self) -> RectSize;
    fn begin_drawing(&self) -> Result<CanvasView, Box<dyn Error>>;
    fn finish_drawing(&self) -> Result<(), Box<dyn Error>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasFormat {
    pub color_format: wgpu::TextureFormat,
    pub depth_stencil_format: Option<wgpu::TextureFormat>,
}

#[derive(Debug, Clone)]
pub struct CanvasView {
    pub color_texture_view: wgpu::TextureView,
    pub depth_stencil_texture_view: Option<wgpu::TextureView>,
    pub logical_size: RectSize,
}

impl CanvasView {
    pub fn projection(&self, space: ProjectionSpace, near: f32, far: f32) -> Matrix4<f32> {
        let w = self.logical_size.width;
        let h = self.logical_size.height;
        let w_half = 0.5 * w;
        let h_half = 0.5 * h;
        use ProjectionSpace::*;
        #[rustfmt::skip]
        let projection = match space {
            //                            left     right   bottom   top
            TopLeftDown  => cgmath::ortho(0.,      w,      h,       0.,      near, far),
            BottomLeftUp => cgmath::ortho(0.,      w,      0.,      h,       near, far),
            CenterDown   => cgmath::ortho(-w_half, w_half, h_half,  -h_half, near, far),
            CenterUp     => cgmath::ortho(-w_half, w_half, -h_half, h_half,  near, far),
        };
        projection
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionSpace {
    TopLeftDown,
    BottomLeftUp,
    CenterDown,
    CenterUp,
}

#[derive(Debug, Clone)]
pub struct TextureCanvas {
    color_texture: wgpu::Texture,
    depth_stencil_texture: Option<wgpu::Texture>,
    format: CanvasFormat,
    logical_size: RectSize,
}

impl TextureCanvas {
    pub fn new(
        color_texture: wgpu::Texture,
        depth_stencil_texture: Option<wgpu::Texture>,
        format: CanvasFormat,
        logical_size: RectSize,
    ) -> Self {
        Self {
            color_texture,
            depth_stencil_texture,
            format,
            logical_size,
        }
    }
}

impl Canvas for TextureCanvas {
    fn format(&self) -> CanvasFormat {
        self.format
    }

    fn logical_size(&self) -> RectSize {
        self.logical_size
    }

    fn begin_drawing(&self) -> Result<CanvasView, Box<dyn Error>> {
        Ok(CanvasView {
            color_texture_view: self.color_texture.create_view(&the_default()),
            depth_stencil_texture_view: self
                .depth_stencil_texture
                .as_ref()
                .map(|texture| texture.create_view(&the_default())),
            logical_size: self.logical_size,
        })
    }

    fn finish_drawing(&self) -> Result<(), Box<dyn Error>> {
        todo!()
    }
}

#[derive(Debug)]
pub struct WindowCanvas<'window> {
    window_surface: wgpu::Surface<'window>,
    depth_stencil_texture: Option<wgpu::Texture>,
    format: CanvasFormat,
    logical_size: RectSize,
    surface_texture: Mutex<Option<wgpu::SurfaceTexture>>,
    surface_config: wgpu::wgt::SurfaceConfiguration<Vec<wgpu::TextureFormat>>,
}

#[derive(Debug, Display, Error)]
pub enum WindowBeginDrawingError {
    #[display("{_0}")]
    SurfaceError(wgpu::SurfaceError),
    #[display(
        "window is currently already being drawn while `begin_drawing` is called (perhaps the last drawing hasn't `finish_drawing` yet?)"
    )]
    IsCurrentlyDrawing,
}

#[derive(Debug, Display, Error)]
pub enum WindowFinishDrawingError {
    #[display("window was not being drawn when `finish_drawing` is called")]
    WasNotDrawing,
}

impl<'window> WindowCanvas<'window> {
    pub fn new(
        window_surface: wgpu::Surface<'window>,
        depth_stencil_texture: Option<wgpu::Texture>,
        format: CanvasFormat,
        logical_size: RectSize,
        surface_config: wgpu::SurfaceConfiguration,
    ) -> Self {
        Self {
            window_surface,
            depth_stencil_texture,
            format,
            logical_size,
            surface_texture: the_default(),
            surface_config,
        }
    }

    pub fn create_for_window(
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        window: Arc<Window>,
        surface_config: impl FnOnce(wgpu::TextureFormat) -> wgpu::SurfaceConfiguration,
    ) -> Self {
        let window_size = window.inner_size();
        let window_scale_factor = window.scale_factor();
        let window_surface = instance.create_surface(window).unwrap();
        let surface_capabilities = window_surface.get_capabilities(adapter);
        log::info!(
            "supported output formats: {:?}",
            surface_capabilities.formats
        );
        let color_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .find(|&format| format.is_srgb())
            .unwrap_or(surface_capabilities.formats[0]);
        log::info!("output color format: {color_format:?}");
        let mut self_ = Self::new(
            window_surface,
            None,
            CanvasFormat {
                color_format,
                depth_stencil_format: None,
            },
            // reconfigure_for_size would initialise this field.
            RectSize::new(0., 0.),
            surface_config(color_format),
        );
        self_.reconfigure_for_size(device, window_size, window_scale_factor, None);
        self_
    }

    pub fn reconfigure_for_size(
        &mut self,
        device: &wgpu::Device,
        size: PhysicalSize<u32>,
        scale_factor: f64,
        new_depth_stencil_texture: Option<wgpu::Texture>,
    ) {
        let logical_size = size.to_logical::<f32>(scale_factor);
        self.logical_size = RectSize::new(logical_size.width, logical_size.height);
        self.surface_config.width = size.width;
        self.surface_config.height = size.height;
        self.window_surface.configure(device, &self.surface_config);
        match (
            self.depth_stencil_texture.as_mut(),
            new_depth_stencil_texture,
        ) {
            (None, None) => (),
            (Some(texture), Some(new_texture)) => *texture = new_texture,
            (None, Some(_)) => panic!(
                "`WindowCanvas::reconfigure_for_size` is provided with a depth stencil texture, but this `WindowCanvas` does not have a depth stencil texture"
            ),
            (Some(_), None) => panic!(
                "`WindowCanvas::reconfigure_for_size` is provided with no depth stencil texture, but this `WindowCanvas` *does have* a depth stencil texture"
            ),
        }
    }
}

impl<'a> Canvas for WindowCanvas<'a> {
    fn format(&self) -> CanvasFormat {
        self.format
    }

    fn logical_size(&self) -> RectSize {
        self.logical_size
    }

    fn begin_drawing(&self) -> Result<CanvasView, Box<dyn Error>> {
        let mut surface_texture_ = self.surface_texture.lock().unwrap();
        if surface_texture_.is_some() {
            return Err(Box::new(WindowBeginDrawingError::IsCurrentlyDrawing));
        }
        let surface_texture = self.window_surface.get_current_texture()?;
        let color_texture_view =
            surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
                    format: Some(self.format.color_format),
                    ..the_default()
                });
        *surface_texture_ = Some(surface_texture);
        let depth_stencil_texture_view = self
            .depth_stencil_texture
            .as_ref()
            .map(|texture| texture.create_view(&the_default()));
        Ok(CanvasView {
            color_texture_view,
            depth_stencil_texture_view,
            logical_size: self.logical_size,
        })
    }

    fn finish_drawing(&self) -> Result<(), Box<dyn Error>> {
        let mut surface_texture = self.surface_texture.lock().unwrap();
        match surface_texture.take() {
            Some(surface_texture) => {
                surface_texture.present();
                Ok(())
            }
            None => Err(Box::new(WindowFinishDrawingError::WasNotDrawing)),
        }
    }
}
