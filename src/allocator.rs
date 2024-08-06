use std::sync::Arc;

use vulkano::{
    DeviceSize,
    device::Device,
    command_buffer::{
        CommandBufferUsage, AutoCommandBufferBuilder, PrimaryAutoCommandBuffer,
        allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo}
    },
    buffer::{
        BufferUsage, Subbuffer,
        allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo}
    },
    memory::allocator::{
        StandardMemoryAllocator, MemoryTypeFilter
    },
    pipeline::graphics::vertex_input::Vertex
};

pub struct Allocator {
    pub command_buffer_allocator: StandardCommandBufferAllocator,
    pub memory_allocator: Arc<StandardMemoryAllocator>,
    pub vertex_buffer_allocator: SubbufferAllocator,
    pub index_buffer_allocator: SubbufferAllocator
}

impl Allocator {
    pub fn new_subbuffer_allocator(
        memory_allocator: Arc<StandardMemoryAllocator>,
        buffer_usage: BufferUsage,
        memory_type_filter: MemoryTypeFilter
    ) -> SubbufferAllocator {
        let create_info = SubbufferAllocatorCreateInfo {
            buffer_usage,
            memory_type_filter,
            ..Default::default()
        };
        SubbufferAllocator::new(memory_allocator, create_info)
    }
    pub fn new(device: Arc<Device>) -> Self {
        let command_buffer_allocator = {
            let create_info = StandardCommandBufferAllocatorCreateInfo::default();
            StandardCommandBufferAllocator::new(device.clone(), create_info)
        };

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

        let vertex_buffer_allocator = Self::new_subbuffer_allocator(
            memory_allocator.clone(),
            BufferUsage::VERTEX_BUFFER,
            MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
        );

        let index_buffer_allocator = Self::new_subbuffer_allocator(
            memory_allocator.clone(),
            BufferUsage::INDEX_BUFFER,
            MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
        );

        Allocator {
            command_buffer_allocator,
            memory_allocator,
            vertex_buffer_allocator,
            index_buffer_allocator
        }
    }
    pub fn alloc_primary_builder(
        &self,
        queue_family_index: u32,
        usage: CommandBufferUsage
    ) -> AutoCommandBufferBuilder<PrimaryAutoCommandBuffer> {
        AutoCommandBufferBuilder::primary(
            &self.command_buffer_allocator,
            queue_family_index,
            usage
        ).expect("Fail to create command buffer builder.")
    }
    pub fn alloc_vertex_buffer<V: Vertex + Clone>(&self, vertices: &Vec<V>) -> Subbuffer<[V]> {
        let vertex_buffer = self.vertex_buffer_allocator.allocate_slice(vertices.len() as DeviceSize)
            .expect("Fail to allocate vertex buffer");
        let mut write_guard = vertex_buffer.write()
            .expect("Fail to obtain write guard of vertex buffer.");
        write_guard.clone_from_slice(vertices.as_slice());
        drop(write_guard);
        vertex_buffer
    }
    pub fn alloc_index_buffer(&self, indices: &Vec<u32>) -> Subbuffer<[u32]> {
        let index_buffer = self.index_buffer_allocator.allocate_slice(indices.len() as DeviceSize)
            .expect("Fail to allocate index buffer");
        let mut write_guard = index_buffer.write()
            .expect("Fail to obtain write guard of index buffer.");
        write_guard.clone_from_slice(indices.as_slice());
        drop(write_guard);
        index_buffer
    }
}