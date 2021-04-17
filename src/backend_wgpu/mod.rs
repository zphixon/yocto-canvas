use bytemuck::{Pod, Zeroable};
use cgmath::Matrix4;
use wgpu::{
    BackendBit, BufferAddress, CommandEncoderDescriptor, Device, DeviceDescriptor, Features,
    InputStepMode, Instance, PresentMode, Queue, RequestAdapterOptions, Surface, SwapChain,
    SwapChainDescriptor, TextureUsage, VertexAttribute, VertexBufferLayout, VertexFormat,
};

pub mod canvas;

use crate::{Context, Result};
use canvas::CanvasPipeline;
use winit::dpi::PhysicalSize;
use winit::window::Window;

pub struct WgpuBackend {
    pub surface: Surface,
    pub device: Device,
    pub queue: Queue,
    pub swapchain: SwapChain,
    pub sc_desc: SwapChainDescriptor,
    pub canvas_pipeline: CanvasPipeline,
    pub updated_uniforms: bool,
}

impl WgpuBackend {
    pub async fn new(window: &Window) -> Result<Self> {
        let size = window.inner_size();
        let instance = Instance::new(BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: Default::default(),
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("device descriptor"),
                    features: Features::empty(),
                    limits: Default::default(),
                },
                None,
            )
            .await
            .context("Couldn't get device")?;

        let sc_desc = SwapChainDescriptor {
            usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::COPY_SRC,
            format: adapter.get_swap_chain_preferred_format(&surface),
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
        };

        let swapchain = device.create_swap_chain(&surface, &sc_desc);

        let canvas_pipeline = CanvasPipeline::new(&device, &queue, &sc_desc)?;

        Ok(WgpuBackend {
            surface,
            device,
            queue,
            swapchain,
            sc_desc,
            canvas_pipeline,
            updated_uniforms: false,
        })
    }

    pub fn update(&mut self, size: &PhysicalSize<u32>, zoom: f32) {
        if !self.updated_uniforms {
            let uniform = Uniform {
                scale_x: self.canvas_pipeline.canvas_image.width() as f32 / size.width as f32,
                scale_y: self.canvas_pipeline.canvas_image.height() as f32 / size.height as f32,
                xform_x: 0.0,
                xform_y: 0.0,
                zoom,
            };

            self.queue.write_buffer(
                &self.canvas_pipeline.canvas_uniform_buffer,
                0,
                bytemuck::cast_slice(&[uniform]),
            );
            self.updated_uniforms = true;
        }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swapchain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    pub fn render(&mut self, size: &PhysicalSize<u32>) -> Result<()> {
        let frame = self.swapchain.get_current_frame()?.output;
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("command encoder"),
            });

        self.canvas_pipeline.execute(
            &mut encoder,
            &self.queue,
            &frame,
            size.width as f32,
            size.height as f32,
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        self.updated_uniforms = false;

        Ok(())
    }
}

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
