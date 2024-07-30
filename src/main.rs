mod debug;
mod framework;
mod resources;
mod renderer;
mod app;

fn main() {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let mut app = app::App::default();
    event_loop.run_app(&mut app).unwrap();
    return;
}