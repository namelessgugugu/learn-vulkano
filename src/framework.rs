use std::{
    sync::Arc,
    collections::HashSet,
    cmp::max
};

use winit::{
    event_loop::ActiveEventLoop,
    window::Window
};

use vulkano::{
    library::VulkanLibrary,
    instance::{
        Instance, InstanceExtensions, InstanceCreateInfo,
        debug::DebugUtilsMessengerCreateInfo
    },
    device::{
        Device, DeviceCreateInfo, Queue, QueueCreateInfo,
        QueueFlags, DeviceExtensions, Features,
        physical::PhysicalDevice
    },
    swapchain::{
        ColorSpace, PresentMode, Surface, SurfaceCapabilities,
        SurfaceInfo, Swapchain, SwapchainCreateInfo, SwapchainAcquireFuture,
        acquire_next_image, SwapchainPresentInfo, PresentFuture
    },
    format::Format,
    image::{
        Image, ImageUsage,ImageSubresourceRange,
        view::{ImageView, ImageViewCreateInfo}
    },
    sync::GpuFuture,
    command_buffer::{
        PrimaryCommandBufferAbstract, CommandBufferExecFuture
    }
};

use crate::debug;

pub struct Framework {
    pub window: Arc<Window>,
    pub instance: Arc<Instance>,
    pub surface: Arc<Surface>,
    pub physical_device: Arc<PhysicalDevice>,
    pub device: Arc<Device>,
    pub graphics_queue: Arc<Queue>,
    pub present_queue: Arc<Queue>,
    pub swapchain: Arc<Swapchain>,
    pub swapchain_images: Vec<Arc<Image>>,
    pub swapchain_image_views: Vec<Arc<ImageView>>
}

impl Framework {
    fn new_window(event_loop: &ActiveEventLoop) -> Arc<Window> {
        let window_attributes = Window::default_attributes();
        Arc::new(event_loop.create_window(window_attributes)
            .expect("Fail to create window."))
    }
    fn new_library() -> Arc<VulkanLibrary> {
        VulkanLibrary::new().expect("Fail to create vulkan library")
    }
    fn library_support(
        library: &Arc<VulkanLibrary>, 
        enabled_layers: &Vec<String>,
        enabled_extensions: &InstanceExtensions,
    ) -> bool {
        let layer_properties = library.layer_properties()
            .expect("Fail to obtain layer properties.");
        let mut supported = vec![false; enabled_layers.len()];
        for layer_property in layer_properties {
            for i in 0..enabled_layers.len() {
                let layer_name = &enabled_layers[i];   
                if layer_property.name() == layer_name {
                    supported[i] = true;
                }
            }
        }
        if supported.iter().any(|p| !p) {
            return false;
        }
        let supported_extensions = library.supported_extensions();
        if !supported_extensions.contains(enabled_extensions) {
            return false;
        }
        true
    }
    fn new_instance(
        library: Arc<VulkanLibrary>,
        enabled_layers: Vec<String>,
        enabled_extensions: InstanceExtensions,
        debug_utils_messengers: Vec<DebugUtilsMessengerCreateInfo>
    ) -> Arc<Instance> {
        if !Self::library_support(&library, &enabled_layers, &enabled_extensions) {
            panic!("Enabled unsupported extensions or layers.");
        }
        let create_info = InstanceCreateInfo {
            enabled_layers,
            enabled_extensions,
            debug_utils_messengers,
            ..Default::default()
        };
        Instance::new(library, create_info).expect("Fail to create vulkan instance")
    }
    fn new_surface(instance: Arc<Instance>, window: Arc<Window>) -> Arc<Surface> {
        Surface::from_window(instance, window).expect("Fail to create surface")
    }
    fn select_physical_device(instance: &Arc<Instance>, filter: impl Fn(&Arc<PhysicalDevice>) -> bool) -> Arc<PhysicalDevice> {
        instance.enumerate_physical_devices()
            .expect("Fail to get available physical devices.")
            .filter(filter)
            .nth(0)
            .expect("Fail to find proper physical device.")
    }
    fn select_graphics_queue_family(physical_device: &Arc<PhysicalDevice>) -> Option<u32> {
        let queue_family_properties = physical_device.queue_family_properties();
        for i in 0..queue_family_properties.len() {
            let property = &queue_family_properties[i];
            if property.queue_flags.contains(QueueFlags::GRAPHICS | QueueFlags::TRANSFER) {
                return Some(i as u32);
            }
        }
        None
    }
    fn select_present_queue_family(physical_device: &Arc<PhysicalDevice>, surface: &Arc<Surface>) -> Option<u32> {
        let queue_family_properties = physical_device.queue_family_properties();
        for i in 0..queue_family_properties.len() {
            if physical_device.surface_support(i as u32, &*surface).unwrap() {
                return Some(i as u32);
            }
        }
        None
    }
    fn get_swapchain_capabilities(physical_device: &Arc<PhysicalDevice>, surface: &Arc<Surface>) -> SurfaceCapabilities {
        physical_device.surface_capabilities(&*surface, SurfaceInfo::default())
            .expect("Fail to get surface capabilities.")
    }
    fn select_swapchain_format(physical_device: &Arc<PhysicalDevice>, surface: &Arc<Surface>) -> Option<(Format, ColorSpace)> {
        let formats = physical_device.surface_formats(&*surface, SurfaceInfo::default())
            .expect("Fail to get available formats");
        for format in formats.iter() {
            if let Format::B8G8R8A8_SRGB = format.0 {
                if let ColorSpace::SrgbNonLinear = format.1 {
                    return Some(*format);
                }
            }
        }
        if formats.is_empty() { None }
        else { Some(formats[0]) }
    }
    fn select_swapchain_present_mode(physical_device: &Arc<PhysicalDevice>, surface: &Arc<Surface>) -> Option<PresentMode> {
        let present_modes = physical_device.surface_present_modes(&*surface, SurfaceInfo::default())
            .expect("Fail to get available presend modes.");
        for mode in present_modes {
            if let PresentMode::Fifo = mode {
                return Some(mode);
            }
        }
        None
    }
    fn physical_device_support(
        physical_device: &Arc<PhysicalDevice>,
        enabled_extensions: &DeviceExtensions,
        enabled_features: &Features
    ) -> bool {
        let supported_extensions = physical_device.supported_extensions();
        if !supported_extensions.contains(enabled_extensions) {
            return false;
        }
        let supported_features = physical_device.supported_features();
        if !supported_features.contains(enabled_features) {
            return false;
        }
        true
    }
    fn new_device(
        physical_device: Arc<PhysicalDevice>,
        queue_create_infos: Vec<QueueCreateInfo>,
        enabled_extensions: DeviceExtensions,
        enabled_features: Features
    ) -> (Arc<Device>, impl ExactSizeIterator<Item = Arc<Queue>>) {
        if !Self::physical_device_support(&physical_device, &enabled_extensions, &enabled_features) {
            panic!("Enabled unsupported device extensions or features.");
        }
        let create_info = DeviceCreateInfo {
            queue_create_infos,
            enabled_extensions,
            enabled_features,
            ..Default::default()
        };
        Device::new(physical_device.clone(), create_info)
            .expect("Fail to create logical device.")
    }
    fn new_swapchain(
        device: Arc<Device>,
        surface: Arc<Surface>,
        format: (Format, ColorSpace),
        present_mode: PresentMode,
        extent: [u32; 2],
        image_count: u32
    ) -> (Arc<Swapchain>, Vec<Arc<Image>>) {
        let create_info = SwapchainCreateInfo {
            image_format: format.0,
            image_color_space: format.1,
            present_mode: present_mode,
            image_extent: extent,
            min_image_count: image_count,
            image_usage: ImageUsage::COLOR_ATTACHMENT,
            ..Default::default()
        };
        Swapchain::new(device, surface, create_info).expect("Fail to create swapchain.")
    }
    fn new_swapchain_image_views(format: Format, swapchain_images: &Vec<Arc<Image>>) -> Vec<Arc<ImageView>> {
        swapchain_images.iter()
            .map(|image| {
                let subresource_range = ImageSubresourceRange::from_parameters(format, 1, 1);
                let create_info = ImageViewCreateInfo {
                    format,
                    subresource_range,
                    ..Default::default()
                };
                ImageView::new(image.clone(), create_info)
                    .expect("Fail to create swapchain image views.")
            })
            .collect()
    }
    pub fn new(event_loop: &ActiveEventLoop) -> Self {
        let window = Self::new_window(event_loop);

        let instance = {
            let library = Self::new_library();

            let enabled_layers = vec![String::from("VK_LAYER_KHRONOS_validation")];
            let enabled_extensions = InstanceExtensions { ext_debug_utils: true, ..Surface::required_extensions(event_loop) };
            let debug_utils_messengers = vec![debug::debug_printing_messenger()];
            Self::new_instance(library, enabled_layers, enabled_extensions, debug_utils_messengers)
        };

        let surface = Self::new_surface(instance.clone(), window.clone());

        
        let enabled_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..Default::default()
        };
        let enabled_features = Features {
            ..Default::default()
        };
        let physical_device = Self::select_physical_device(
            &instance,
            |physical_device| -> bool {
                Self::select_graphics_queue_family(physical_device).is_some()
                && Self::select_present_queue_family(physical_device, &surface).is_some()
                && Self::select_swapchain_format(physical_device, &surface).is_some()
                && Self::select_swapchain_present_mode(physical_device, &surface).is_some()
                && Self::physical_device_support(physical_device, &enabled_extensions, &enabled_features)
            }
        );

        let (device, graphics_queue, present_queue) = {
            let graphics_queue_family_index = Self::select_graphics_queue_family(&physical_device)
                .expect("[?]Fail to find graphics family index.");
            let present_queue_family_index = Self::select_present_queue_family(&physical_device, &surface)
                .expect("[?]Fail to find present family index.");
            let unique_indices = HashSet::from([graphics_queue_family_index, present_queue_family_index]);
            let queue_create_infos = unique_indices
                .iter()
                .map(|index| QueueCreateInfo { queue_family_index: *index, ..Default::default() })
                .collect();
            let (device, queues) = Self::new_device(physical_device.clone(), queue_create_infos, enabled_extensions, enabled_features);
            let queues = queues.collect::<Vec<_>>();
            let retrieve_queue = |index: u32| -> Arc<Queue> {
                for queue in queues.iter() {
                    if queue.queue_family_index() == index {
                        return queue.clone();
                    }
                }
                panic!("[?]Fail to find corresponding queue.");
            };
            let graphics_queue = retrieve_queue(graphics_queue_family_index);
            let present_queue = retrieve_queue(present_queue_family_index);
            (device, graphics_queue, present_queue)
        };

        let (swapchain, swapchain_images) = {
            let format = Self::select_swapchain_format(&physical_device, &surface)
                .expect("[?]Fail to select format");
            let present_mode = Self::select_swapchain_present_mode(&physical_device, &surface)
                .expect("[?]Fail to select present mode");
            let capabilities = Self::get_swapchain_capabilities(&physical_device, &surface);
            let extent = capabilities.current_extent.unwrap();
            let image_count = if let Some(max_image_count) = capabilities.max_image_count {
                max(max_image_count, capabilities.min_image_count + 1)
            }
            else {
                capabilities.min_image_count + 1
            };
            Self::new_swapchain(device.clone(), surface.clone(), format, present_mode, extent, image_count)
        };

        let swapchain_image_views = Self::new_swapchain_image_views(swapchain.image_format(), &swapchain_images);

        Framework {
            window,
            instance,
            surface,
            physical_device,
            device,
            graphics_queue,
            present_queue,
            swapchain,
            swapchain_images,
            swapchain_image_views
        }
    }
    pub fn recreate_swapchain(&mut self) -> bool {
        let (swapchain, swapchain_images) = {
            let capabilities = Self::get_swapchain_capabilities(&self.physical_device, &self.surface);
            let extent = capabilities.current_extent.unwrap();
            if extent[0] == 0 || extent[1] == 0 {
                return false;
            }
            let create_info = SwapchainCreateInfo {
                image_extent: extent,
                ..self.swapchain.create_info()
            };
            self.swapchain.recreate(create_info).expect("Fail to recreate swapchain.")
        };
        let swapchain_image_views = Self::new_swapchain_image_views(swapchain.image_format(), &swapchain_images);

        self.swapchain_image_views = swapchain_image_views;
        self.swapchain_images = swapchain_images;
        self.swapchain = swapchain;
        true
    }
    pub fn acquire_next_image(&self) -> Option<(u32, SwapchainAcquireFuture)> {
        let (image_index, suboptimal, image_available) = acquire_next_image(self.swapchain.clone(), None)
            .expect("Fail to acquire next image.");
        if suboptimal { None }
        else { Some((image_index, image_available)) }
    }
    pub fn execute_command_buffer<F, C>(&self, before: F, command_buffer: Arc<C>) -> CommandBufferExecFuture<F>
    where 
        F: GpuFuture,
        C: 'static + PrimaryCommandBufferAbstract
    {
        before.then_execute(self.graphics_queue.clone(), command_buffer)
            .expect("Fail to execute command buffer.")
    }
    pub fn present_image<F: GpuFuture>(&self, before: F, image_index: u32) -> PresentFuture<F> {
        let swapchain_info = SwapchainPresentInfo::swapchain_image_index(
            self.swapchain.clone(),
            image_index
        );
        before.then_swapchain_present(self.present_queue.clone(), swapchain_info)
    }
}