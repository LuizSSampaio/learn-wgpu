use winit::event_loop::{ControlFlow, EventLoop};

mod app;
mod camera;
mod instance;
mod model;
mod renderer;
mod resources;
mod texture;
mod wgpu_utils;

use app::App;

fn main() -> Result<(), winit::error::EventLoopError> {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app)
}
