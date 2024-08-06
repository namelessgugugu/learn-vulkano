mod debug;
mod framework;
mod model;
mod allocator;
mod renderer;
mod app;

fn main() {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let mut app = app::OptionApp::default();
    event_loop.run_app(&mut app).unwrap();
    return;
}