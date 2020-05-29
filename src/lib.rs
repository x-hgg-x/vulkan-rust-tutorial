pub const WIDTH: u32 = 800;
pub const HEIGHT: u32 = 800;

#[derive(Default, Debug, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 3],
}
vulkano::impl_vertex!(Vertex, position, color);

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
