pub use anyhow::{Context, Result};

use bytemuck::{Pod, Zeroable};

use cgmath::Matrix4;

use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use wgpu::{
    BackendBit, BufferAddress, CommandEncoderDescriptor, Device, DeviceDescriptor, Features,
    InputStepMode, Instance, PresentMode, Queue, RequestAdapterOptions, Surface, SwapChain,
    SwapChainDescriptor, SwapChainError, TextureUsage, VertexAttribute, VertexBufferLayout,
    VertexFormat,
};

mod backend_wgpu;
mod composite;
mod image;
mod texture;

use crate::{backend_wgpu::canvas::CanvasPipeline, image::Pixel};

#[rustfmt::skip]
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

const VERTICES: [Vertex; 6] = [
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
struct Vertex {
    position: [f32; 2],
    tex_coord: [f32; 2],
}

impl Vertex {
    fn desc<'a>() -> VertexBufferLayout<'a> {
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
struct Uniform {
    scale_x: f32,
    scale_y: f32,
    xform_x: f32,
    xform_y: f32,
    zoom: f32,
}

#[derive(Debug)]
struct Mouse {
    x: f32,
    y: f32,
    left: ElementState,
    right: ElementState,
}

#[allow(dead_code)]
struct State {
    surface: Surface,
    device: Device,
    queue: Queue,
    swapchain: SwapChain,
    sc_desc: SwapChainDescriptor,
    size: PhysicalSize<u32>,
    canvas_pipeline: CanvasPipeline,
    mouse: Mouse,
    zoom: f32,
    updated_uniforms: bool,
}

impl State {
    async fn new(window: &Window) -> Result<Self> {
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

        let mouse = Mouse {
            x: size.width as f32 / 2.,
            y: size.height as f32 / 2.,
            left: ElementState::Released,
            right: ElementState::Released,
        };

        let zoom = 1.0;

        let updated_uniforms = false;

        Ok(Self {
            surface,
            device,
            queue,
            swapchain,
            sc_desc,
            size,
            canvas_pipeline,
            mouse,
            zoom,
            updated_uniforms,
        })
    }

    // returns true if state captured the event, false otherwise
    // redraws if returns true
    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::MouseInput { state, button, .. } => {
                match button {
                    MouseButton::Left => self.mouse.left = *state,
                    MouseButton::Right => self.mouse.right = *state,
                    _ => {}
                }

                true
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse.x = position.x as f32;
                self.mouse.y = position.y as f32;
                self.mouse.left == ElementState::Pressed
                    || self.mouse.right == ElementState::Pressed
            }
            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::LineDelta(_x, y),
                ..
            } => {
                self.zoom = (self.zoom + y.signum()).clamp(1.0, 10.0);
                true
            }
            _ => false,
        }
    }

    fn update(&mut self) {
        if self.mouse.left == ElementState::Pressed {
            self.canvas_pipeline.canvas_image.set_pixel(
                40,
                20,
                Pixel {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                },
            );
        } else {
            self.canvas_pipeline.canvas_image.set_pixel(
                40,
                20,
                Pixel {
                    r: 1.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                },
            );
        }

        if !self.updated_uniforms {
            let uniform = Uniform {
                scale_x: self.canvas_pipeline.canvas_image.width() as f32 / self.size.width as f32,
                scale_y: self.canvas_pipeline.canvas_image.height() as f32
                    / self.size.height as f32,
                xform_x: 0.0,
                xform_y: 0.0,
                zoom: self.zoom,
            };

            self.queue.write_buffer(
                &self.canvas_pipeline.canvas_uniform_buffer,
                0,
                bytemuck::cast_slice(&[uniform]),
            );
            self.updated_uniforms = true;
        }
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swapchain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    fn render(&mut self) -> Result<()> {
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
            self.size.width as f32,
            self.size.height as f32,
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        self.updated_uniforms = false;

        Ok(())
    }
}

fn main() -> Result<()> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop)?;
    window.set_inner_size(PhysicalSize {
        width: 800,
        height: 675,
    });

    let mut state = futures::executor::block_on(State::new(&window))?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if state.input(&event) {
                    state.update();
                    window.request_redraw();
                } else {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        // TODO remove later
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(size) => {
                            state.resize(*size);
                            state.update();
                            window.request_redraw();
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            state.resize(**new_inner_size);
                            state.update();
                            window.request_redraw();
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(window_id) if window_id == window.id() => match state.render() {
                Ok(_) => {}
                Err(e) => match e.downcast::<SwapChainError>() {
                    Ok(e) => match e {
                        SwapChainError::Lost => {}
                        SwapChainError::OutOfMemory => *control_flow = ControlFlow::Exit,
                        e => println!("{}", e),
                    },
                    Err(e) => println!("{}", e),
                },
            },
            _ => {}
        }
    });
}
