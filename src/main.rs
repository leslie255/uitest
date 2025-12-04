#![allow(dead_code)]

extern crate derive;

pub mod app;
pub mod wgpu_utils;

pub mod resources;

pub mod rendering;

pub(crate) mod utils;

use std::path::PathBuf;

use winit::event_loop::EventLoop;

use crate::{app::Application, resources::AppResources};

fn main() {
    env_logger::init();

    // TODO: read resource directory path from command line args.
    let resources = AppResources::new(PathBuf::from("res/"));
    let event_loop = EventLoop::builder().build().unwrap();
    event_loop
        .run_app(&mut Application::new(&resources))
        .unwrap();
}
