use vulkano::{
    buffer::BufferContents,
    pipeline::graphics::vertex_input::Vertex
};

#[derive(Clone)]
#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct ColoredVertex {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
    #[format(R32G32B32_SFLOAT)]
    pub color: [f32; 3]
}
impl ColoredVertex {
    pub fn new(position: [f32; 3], color: [f32; 3]) -> Self {
        ColoredVertex { position, color }
    }
}