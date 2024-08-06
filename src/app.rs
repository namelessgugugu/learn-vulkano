use winit::{
    application::ApplicationHandler,
    event_loop::ActiveEventLoop,
    window::WindowId,
    dpi::PhysicalSize,
    event::WindowEvent
};

use vulkano::sync::GpuFuture;

use crate::{
    framework::Framework,
    allocator::Allocator,
    model::ColoredVertex,
    renderer::Renderer
};

pub struct App {
    pub framework: Framework,
    pub allocator: Allocator,
    pub renderer: Renderer,
    pub minimized: bool
}
impl App {
    fn new(event_loop: &ActiveEventLoop) -> Self {
        let framework = Framework::new(event_loop);
        let format = framework.swapchain.image_format();
        let allocator = Allocator::new(framework.device.clone());
        let renderer = Renderer::new(framework.device.clone(), format);
        App {
            framework,
            allocator,
            renderer,
            minimized: false
        }
    }
    fn draw_frame(&mut self) -> bool {
        let framework = &mut self.framework;
        let allocator = &self.allocator;
        let renderer = &self.renderer;
        let (image_index, image_available) = {
            let mut current_info = framework.acquire_next_image();
            if current_info.is_none() {
                if framework.recreate_swapchain() {
                    current_info = framework.acquire_next_image();
                }
            }
            if current_info.is_some() { current_info.unwrap() }
            else { return false; }
        };

        let vertices = vec![
            ColoredVertex::new([-0.5, -0.5, 0.0], [0.2, 0.6, 0.9]),
            ColoredVertex::new([-0.5, 0.5, 0.0], [0.9, 0.5, 0.65]),
            ColoredVertex::new([0.5, -0.5, 0.0], [0.9, 0.5, 0.65]),
            ColoredVertex::new([0.5, 0.5, 0.0], [1.0, 1.0, 1.0])
        ];
        let vertex_buffer = allocator.alloc_vertex_buffer(&vertices);
        let indices = vec![0, 1, 2, 2, 1, 3];
        let index_buffer = allocator.alloc_index_buffer(&indices);

        let command_buffer = renderer.record_command_buffer(
            allocator,
            framework.graphics_queue.queue_family_index(),
            vertex_buffer,
            index_buffer,
            indices.len() as u32,
            framework.swapchain_image_views[image_index as usize].clone()
        );
        
        let render_finished = framework.execute_command_buffer(image_available, command_buffer)
            .then_signal_semaphore_and_flush()
            .expect("Fail to flush render finished future.");

        let presented = framework.present_image(render_finished, image_index)
            .then_signal_fence_and_flush()
            .expect("Fail to flush presented future.");

        presented.wait(None)
            .expect("Fail to wait for presenting.");
        
        framework.window.request_redraw();
        true
    }
}

#[derive(Default)]
pub struct OptionApp(Option<App>);

impl ApplicationHandler for OptionApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.0 = Some(App::new(event_loop));
    }
    fn window_event(
            &mut self,
            _event_loop: &ActiveEventLoop,
            _window_id: WindowId,
            event: WindowEvent,
        ) {
        eprintln!("new event: {event:?}");
        use WindowEvent::*;
        match event {
            CloseRequested => {
                self.0.take();
            }
            Resized(PhysicalSize { width, height }) => {
                let app = self.0.as_mut().unwrap();
                if width == 0 || height == 0 {
                    app.minimized = true;
                }
                else {
                    app.framework.recreate_swapchain();
                    app.minimized = false;
                }   
            }
            RedrawRequested => {
                let app = self.0.as_mut().unwrap();
                if !app.minimized && !app.draw_frame() {
                    app.minimized = true;
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.0.is_none()
        { event_loop.exit(); }
    }
}