use winit::application::ApplicationHandler;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;
use winit::event::WindowEvent;
use winit::dpi::PhysicalSize;

use vulkano::sync::GpuFuture;

use crate::framework::Framework;
use crate::resources::Resources;
use crate::renderer::Renderer;
#[derive(Default)]
pub struct App {
    framework: Option<Framework>,
    resources: Option<Resources>,
    renderer: Option<Renderer>,
    minimized: bool
}
impl App {
    fn draw_frame(&mut self) -> bool {
        let framework = self.framework.as_mut().unwrap();
        let resources = self.resources.as_ref().unwrap();
        let renderer = self.renderer.as_ref().unwrap();
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
        eprintln!("before creating cb");
        let command_buffer = renderer.record_command_buffer(
            resources,
            framework.swapchain_image_views[image_index as usize].clone(),
            framework.graphics_queue.queue_family_index()
        );
        eprintln!("after creating cb");
        let render_finished = framework.execute_command_buffer(image_available, command_buffer);
        let presented = framework.present_image(render_finished, image_index);
        presented.flush().expect("Fail to flush gpu future.");
        true
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.framework = Some(Framework::new(event_loop));
        let framework = self.framework.as_ref().unwrap();
        let format = framework.swapchain.image_format();
        self.resources = Some(Resources::new(framework.device.clone()));
        self.renderer = Some(Renderer::new(framework.device.clone(), format));
    }
    fn window_event(
            &mut self,
            _event_loop: &ActiveEventLoop,
            _window_id: WindowId,
            event: WindowEvent,
        ) {
        eprintln!("new event: {event:?}");
        use WindowEvent::*;
        match event
        {
            CloseRequested => {
                self.renderer.take();
                self.resources.take();
                self.framework.take();
            }
            Resized(PhysicalSize { width, height }) => {
                if width == 0 && height == 0 {
                    self.minimized = true;
                }
                else {
                    self.framework.as_mut().unwrap().recreate_swapchain();
                    self.minimized = false;
                }   
            }
            RedrawRequested => {
                if !self.minimized {
                    if !self.draw_frame() {
                        self.minimized = true;
                    }
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop)
    {
        if self.framework.is_none()
        { event_loop.exit(); }
    }
}