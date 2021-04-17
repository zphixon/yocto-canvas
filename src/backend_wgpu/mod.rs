use bytemuck::{Pod, Zeroable};
use cgmath::Matrix4;
use wgpu::{BufferAddress, InputStepMode, VertexAttribute, VertexBufferLayout, VertexFormat};

pub mod canvas;

#[rustfmt::skip]
#[allow(dead_code)]
pub const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

//    top left             top right
// xy -1 -1                1 -1
// uv  0  1                1  1
// xy -1  1                1  1
// uv  0  0                1  0
//    bottom left          bottom right

pub const VERTICES: [Vertex; 6] = [
    // top left
    Vertex {
        position: [-1., -1.],
        tex_coord: [0., 1.],
    },
    // top right
    Vertex {
        position: [1., -1.],
        tex_coord: [1., 1.],
    },
    // bottom right
    Vertex {
        position: [1., 1.],
        tex_coord: [1., 0.],
    },
    // bottom right
    Vertex {
        position: [1., 1.],
        tex_coord: [1., 0.],
    },
    // bottom left
    Vertex {
        position: [-1., 1.],
        tex_coord: [0., 0.],
    },
    // top left
    Vertex {
        position: [-1., -1.],
        tex_coord: [0., 1.],
    },
];

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub tex_coord: [f32; 2],
}

impl Vertex {
    pub fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
            step_mode: InputStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float2,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float2,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Uniform {
    pub scale_x: f32,
    pub scale_y: f32,
    pub xform_x: f32,
    pub xform_y: f32,
    pub zoom: f32,
}
