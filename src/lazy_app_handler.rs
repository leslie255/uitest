use std::marker::PhantomData;

use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::WindowId,
};

/// Like `winit::application::ApplicationHandler`, but lazy initialized on first `resume`.
///
/// This is useful for apps with one main window, because on initialization it can be provided a
/// `winit::event_loop::ActiveEventLoop`, with which the main window can be created.
pub trait LazyApplicationHandler<InitContext = (), UserEvent: 'static = ()>:
    ApplicationHandler<UserEvent>
{
    fn new(init_context: InitContext, event_loop: &ActiveEventLoop) -> Self;
}

pub trait EventLoopExt<UserEvent: 'static> {
    fn run_lazy_initialized_app<App, InitContext>(
        self,
        init_context: InitContext,
    ) -> Result<(), EventLoopError>
    where
        App: LazyApplicationHandler<InitContext, UserEvent>;
}

impl<UserEvent: 'static> EventLoopExt<UserEvent> for EventLoop<UserEvent> {
    fn run_lazy_initialized_app<App, InitContext>(
        self,
        init_context: InitContext,
    ) -> Result<(), EventLoopError>
    where
        App: LazyApplicationHandler<InitContext, UserEvent>,
    {
        let mut app_runner = LazyAppRunner::<App, InitContext, UserEvent>::new(init_context);
        self.run_app(&mut app_runner)
    }
}

struct LazyAppRunner<App, InitContext, UserEvent> {
    init_context: Option<InitContext>,
    app: Option<App>,
    _marker: PhantomData<UserEvent>,
}

impl<App, InitContext, UserEvent> LazyAppRunner<App, InitContext, UserEvent> {
    fn new(init_context: InitContext) -> Self {
        Self {
            init_context: Some(init_context),
            app: None,
            _marker: PhantomData,
        }
    }
}

impl<App, InitContext, UserEvent> ApplicationHandler<UserEvent>
    for LazyAppRunner<App, InitContext, UserEvent>
where
    UserEvent: 'static,
    App: LazyApplicationHandler<InitContext, UserEvent>,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let app = self.app.get_or_insert_with(|| {
            let init_context = self.init_context.take().unwrap();
            App::new(init_context, event_loop)
        });
        app.resumed(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(app) = self.app.as_mut() {
            app.window_event(event_loop, window_id, event);
        }
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        if let Some(app) = self.app.as_mut() {
            app.new_events(event_loop, cause);
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        if let Some(app) = self.app.as_mut() {
            app.user_event(event_loop, event);
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let Some(app) = self.app.as_mut() {
            app.device_event(event_loop, device_id, event);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(app) = self.app.as_mut() {
            app.about_to_wait(event_loop);
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(app) = self.app.as_mut() {
            app.suspended(event_loop);
        }
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(app) = self.app.as_mut() {
            app.exiting(event_loop);
        }
    }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(app) = self.app.as_mut() {
            app.memory_warning(event_loop);
        }
    }
}
