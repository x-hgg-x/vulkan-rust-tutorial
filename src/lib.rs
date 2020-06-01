use std::sync::Arc;
use vulkano::buffer::ImmutableBuffer;

pub const WIDTH: u32 = 800;
pub const HEIGHT: u32 = 600;

#[derive(Default, Debug, Clone)]
pub struct Vertex {
    pub position: [f32; 3],
    pub texture_coords: [f32; 2],
}
vulkano::impl_vertex!(Vertex, position, texture_coords);

pub type VertexBuffer = Arc<ImmutableBuffer<[Vertex]>>;
pub type IndexBuffer = Arc<ImmutableBuffer<[u32]>>;

pub mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/shader.vert"
    }
}

pub mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/shader.frag"
    }
}
