use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    sync::Mutex,
};

use derive_more::{Display, Error, From};
use image::{ImageError, RgbaImage};
use serde::de::DeserializeOwned;

use crate::utils::*;

#[derive(Debug, Display, From, Error)]
pub enum LoadResourceError {
    #[display(
        "resource of another type `{other_type:?}` had aready been loaded from path {path:?} while trying to load resource of type `{this_type:?}` in the same path"
    )]
    TypeConflict {
        path: PathBuf,
        this_type: ResourceType,
        other_type: ResourceType,
    },
    #[display("{_0}")]
    ImageError(ImageError),
    #[display("{_0}")]
    IoError(io::Error),
    #[display("{_0}")]
    SerdeJsonError(serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Text,
    Image,
    Shader,
}

#[derive(Clone)]
enum Resource {
    Text(Box<str>),
    Image(Box<RgbaImage>),
    Shader(Box<wgpu::ShaderModule>),
}

impl Resource {
    pub fn type_(&self) -> ResourceType {
        match self {
            Resource::Shader(_) => ResourceType::Shader,
            Resource::Text(_) => ResourceType::Text,
            Resource::Image(_) => ResourceType::Image,
        }
    }
}

pub struct AppResources {
    resource_directory: PathBuf,
    loaded_resources: Mutex<HashMap<PathBuf, Resource>>,
}

impl AppResources {
    pub fn new(resource_directory: PathBuf) -> Self {
        Self {
            resource_directory,
            loaded_resources: the_default(),
        }
    }

    pub fn load_text(&self, subpath: impl AsRef<Path>) -> Result<&str, LoadResourceError> {
        let path = self.resource_directory.join(subpath.as_ref());
        let mut loaded_resources = self.loaded_resources.lock().unwrap();
        if let Some(cached_resource) = loaded_resources.get(&path) {
            let cached_text: &str = match cached_resource {
                Resource::Text(text) => text.as_ref(),
                resource => {
                    return Err(LoadResourceError::TypeConflict {
                        path,
                        this_type: ResourceType::Text,
                        other_type: resource.type_(),
                    });
                }
            };
            return Ok(unsafe { transmute_lifetime(cached_text) });
        }
        log::info!("loading resource {path:?}...");
        let text: Box<str> = fs::read_to_string(&path)?.into();
        let ptr: *const str = text.as_ref() as *const _;
        loaded_resources.insert(path, Resource::Text(text));
        Ok(unsafe { &*ptr })
    }

    pub fn load_image(&self, subpath: impl AsRef<Path>) -> Result<&RgbaImage, LoadResourceError> {
        let path = self.resource_directory.join(subpath.as_ref());
        let mut loaded_resources = self.loaded_resources.lock().unwrap();
        if let Some(cached_resource) = loaded_resources.get(&path) {
            let cached_shader: &RgbaImage = match cached_resource {
                Resource::Image(image) => image.as_ref(),
                resource => {
                    return Err(LoadResourceError::TypeConflict {
                        path,
                        this_type: ResourceType::Image,
                        other_type: resource.type_(),
                    });
                }
            };
            return Ok(unsafe { transmute_lifetime(cached_shader) });
        }
        log::info!("loading resource {path:?}...");
        let image_boxed = Box::new(image::open(&path)?.into_rgba8());
        let ptr: *const RgbaImage = image_boxed.as_ref() as *const _;
        loaded_resources.insert(path, Resource::Image(image_boxed));
        Ok(unsafe { &*ptr })
    }

    pub fn load_shader(
        &self,
        subpath: impl AsRef<Path>,
        device: &wgpu::Device,
    ) -> Result<&wgpu::ShaderModule, LoadResourceError> {
        let path = self.resource_directory.join(subpath.as_ref());
        let mut loaded_resources = self.loaded_resources.lock().unwrap();
        if let Some(cached_resource) = loaded_resources.get(&path) {
            let cached_shader: &wgpu::ShaderModule = match cached_resource {
                Resource::Shader(shader) => shader.as_ref(),
                resource => {
                    return Err(LoadResourceError::TypeConflict {
                        path,
                        this_type: ResourceType::Shader,
                        other_type: resource.type_(),
                    });
                }
            };
            return Ok(unsafe { transmute_lifetime(cached_shader) });
        }
        log::info!("loading resource {path:?}...");
        let source = fs::read_to_string(&path)?;
        let shader_boxed = Box::new(device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(source.into()),
        }));
        let ptr: *const wgpu::ShaderModule = shader_boxed.as_ref() as *const _;
        loaded_resources.insert(path, Resource::Shader(shader_boxed));
        Ok(unsafe { &*ptr })
    }

    pub fn load_json_object<T: DeserializeOwned>(
        &self,
        subpath: impl AsRef<Path>,
    ) -> Result<T, LoadResourceError> {
        let text = self.load_text(&subpath)?;
        Ok(serde_json::from_str(text)?)
    }

    /// Returns a new subpath.
    pub fn solve_relative_subpath(
        &self,
        subpath: impl AsRef<Path>,
        relative_path: impl AsRef<Path>,
    ) -> PathBuf {
        let origin_subpath = PathBuf::from(subpath.as_ref());
        match origin_subpath.parent() {
            Some(path) => path.join(relative_path),
            None => relative_path.as_ref().into(),
        }
    }
}
