use std::sync::Arc;

use vulkano::device::Device;
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};

#[derive(Debug)]
pub struct Resources {
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>
}

impl Resources {
    pub fn new(device: Arc<Device>) -> Self {
        let command_buffer_allocator = Arc::new(
            {
                let create_info = StandardCommandBufferAllocatorCreateInfo::default();
                StandardCommandBufferAllocator::new(device.clone(), create_info)
            }
        );
        Resources {
            command_buffer_allocator
        }
    }
}