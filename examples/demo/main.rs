use std::path::PathBuf;

use winit::event_loop::EventLoop;

use muilib::{AppResources, EventLoopExt as _};

use crate::app::App;

mod app;
mod theme;

fn main() {
    env_logger::init();

    // TODO: read resource directory path from command line args.
    let resources = AppResources::new(PathBuf::from("res/"));
    let event_loop = EventLoop::builder().build().unwrap();
    event_loop
        .run_lazy_initialized_app::<App, _>(&resources)
        .unwrap();
}
