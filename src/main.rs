pub use anyhow::{Context, Result};

use bytemuck::{Pod, Zeroable};

use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BackendBit, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendState, Buffer, BufferAddress,
    BufferUsage, ColorTargetState, ColorWrite, CommandEncoderDescriptor, CullMode, Device,
    DeviceDescriptor, Features, FragmentState, FrontFace, InputStepMode, Instance, LoadOp,
    MultisampleState, Operations, PipelineLayoutDescriptor, PolygonMode, PresentMode,
    PrimitiveState, PrimitiveTopology, Queue, RenderPassColorAttachmentDescriptor,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions,
    ShaderStage, Surface, SwapChain, SwapChainDescriptor, SwapChainError, TextureSampleType,
    TextureUsage, TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexState,
};

mod texture;

use texture::MyTexture;

//const vec2 corners[6] = vec2[6](
//    vec2(-1.0, -1.0),
//    vec2(1.0, -1.0),
//    vec2(1.0, 1.0),
//
//    vec2(1.0, 1.0),
//    vec2(-1.0, 1.0),
//    vec2(-1.0, -1.0)
//);

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

struct State {
    surface: Surface,
    device: Device,
    queue: Queue,
    swapchain: SwapChain,
    sc_desc: SwapChainDescriptor,
    size: PhysicalSize<u32>,
    pipeline: RenderPipeline,
    texture: MyTexture,
    vertex_buffer: Buffer,
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
            usage: TextureUsage::RENDER_ATTACHMENT,
            format: adapter.get_swap_chain_preferred_format(&surface),
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
        };

        let swapchain = device.create_swap_chain(&surface, &sc_desc);

        let texture = MyTexture::empty(&device, &queue, "image")?;

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pipeline layout"),
            bind_group_layouts: &[&texture.layout],
            push_constant_ranges: &[],
        });

        let vs_module =
            device.create_shader_module(&wgpu::include_spirv!("../shaders/shader.vert.spv"));
        let fs_module =
            device.create_shader_module(&wgpu::include_spirv!("../shaders/shader.frag.spv"));

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &vs_module,
                entry_point: "main",
                buffers: &[Vertex::desc()],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Cw,
                cull_mode: CullMode::None,
                polygon_mode: PolygonMode::Fill,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                module: &fs_module,
                entry_point: "main",
                targets: &[ColorTargetState {
                    format: sc_desc.format,
                    alpha_blend: BlendState::REPLACE,
                    color_blend: BlendState::REPLACE,
                    write_mask: ColorWrite::ALL,
                }],
            }),
        });

        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("vertex buffer"),
            contents: bytemuck::cast_slice(&VERTICES),
            usage: BufferUsage::VERTEX,
        });

        Ok(Self {
            surface,
            device,
            queue,
            swapchain,
            sc_desc,
            size,
            pipeline,
            texture,
            vertex_buffer,
        })
    }

    // returns true if state captured the event, false otherwise
    fn input(&mut self, _event: &WindowEvent) -> bool {
        false
    }

    fn render(&mut self) -> Result<()> {
        let frame = self.swapchain.get_current_frame()?.output;
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("command encoder"),
            });

        {
            let mut rp = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            rp.set_pipeline(&self.pipeline);

            rp.set_bind_group(0, &self.texture.group, &[]);
            //rp.set_bind_group(1, self.uniform)

            rp.set_vertex_buffer(0, self.vertex_buffer.slice(..));

            let len = VERTICES.len() as u32;
            rp.draw(0..len, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}

fn main() -> Result<()> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop)?;

    let mut state = futures::executor::block_on(State::new(&window))?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() && !state.input(event) => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => {}
            },
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                //state.update();
                match state.render() {
                    Ok(_) => {}
                    Err(e) => match e.downcast::<SwapChainError>() {
                        Ok(e) => match e {
                            SwapChainError::Lost => {}
                            SwapChainError::OutOfMemory => *control_flow = ControlFlow::Exit,
                            e => println!("{}", e),
                        },
                        Err(e) => println!("{}", e),
                    },
                }
            }
            _ => {}
        }
    });
}
