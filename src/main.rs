pub use anyhow::Result;

use cgmath::{Deg, InnerSpace, Matrix4, Quaternion, Rotation3, Vector3, Zero};

use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

// i don't want to type 'wgpu' a thousand times thank you very much
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BackendBit, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendState, Buffer, BufferAddress,
    BufferBindingType, BufferUsage, Color, ColorTargetState, ColorWrite, CommandEncoderDescriptor,
    CompareFunction, CullMode, DepthBiasState, DepthStencilState, Device, DeviceDescriptor,
    Features, FragmentState, FrontFace, IndexFormat, InputStepMode, Instance, Limits, LoadOp,
    MultisampleState, Operations, PipelineLayoutDescriptor, PolygonMode, PowerPreference,
    PresentMode, PrimitiveState, PrimitiveTopology, Queue, RenderPassColorAttachmentDescriptor,
    RenderPassDepthStencilAttachmentDescriptor, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, RequestAdapterOptions, ShaderStage, StencilState, Surface, SwapChain,
    SwapChainDescriptor, SwapChainError, TextureSampleType, TextureUsage, TextureViewDimension,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexState,
};

mod camera;
mod texture;

const NUM_INSTANCES_PER_ROW: u32 = 10;
const NUM_INSTANCES: u32 = NUM_INSTANCES_PER_ROW * NUM_INSTANCES_PER_ROW;
const INSTANCE_DISPLACEMENT: Vector3<f32> = Vector3::new(
    NUM_INSTANCES_PER_ROW as f32 * 0.5,
    0.0,
    NUM_INSTANCES_PER_ROW as f32 * 0.5,
);

struct MyInstance {
    position: Vector3<f32>,
    rotation: Quaternion<f32>,
}

impl MyInstance {
    fn to_raw(&self) -> InstanceRawXd {
        InstanceRawXd {
            model: (Matrix4::from_translation(self.position) * Matrix4::from(self.rotation)).into(),
        }
    }
}

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct InstanceRawXd {
    model: [[f32; 4]; 4],
}

impl InstanceRawXd {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        use std::mem::size_of;
        VertexBufferLayout {
            array_stride: size_of::<InstanceRawXd>() as BufferAddress,
            step_mode: InputStepMode::Instance,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: VertexFormat::Float4,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 4]>() as BufferAddress,
                    shader_location: 6,
                    format: VertexFormat::Float4,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 8]>() as BufferAddress,
                    shader_location: 7,
                    format: VertexFormat::Float4,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 12]>() as BufferAddress,
                    shader_location: 8,
                    format: VertexFormat::Float4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
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
                    format: VertexFormat::Float3,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float2,
                },
            ],
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.0868241, 0.49240386, 0.0],
        tex_coords: [0.4131759, 0.99240386],
    }, // A
    Vertex {
        position: [-0.49513406, 0.06958647, 0.0],
        tex_coords: [0.0048659444, 0.56958646],
    }, // B
    Vertex {
        position: [-0.21918549, -0.44939706, 0.0],
        tex_coords: [0.28081453, 0.050602943],
    }, // C
    Vertex {
        position: [0.35966998, -0.3473291, 0.0],
        tex_coords: [0.85967, 0.15267089],
    }, // D
    Vertex {
        position: [0.44147372, 0.2347359, 0.0],
        tex_coords: [0.9414737, 0.7347359],
    }, // E
];

const INDICES_PENTAGON: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
}

impl Uniforms {
    fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, view_proj: Matrix4<f32>) {
        self.view_proj = view_proj.into();
    }
}

struct State {
    surface: Surface,
    device: Device,
    queue: Queue,
    sc_desc: SwapChainDescriptor,
    swapchain: SwapChain,
    size: PhysicalSize<u32>,
    render_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer_pentagon: Buffer,
    num_indices_pentagon: u32,
    diffuse_bind_group: BindGroup,
    diffuse_texture: texture::MyTexture,
    depth_texture: texture::MyTexture,
    camera: camera::Camera,
    uniforms: Uniforms,
    uniform_buffer: Buffer,
    uniform_bind_group: BindGroup,
    instances: Vec<MyInstance>,
    instance_buffer: Buffer,
}

impl State {
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();
        let instance = Instance::new(BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    features: Features::empty(),
                    limits: Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        println!("adapter supports");
        println!("{:?}", adapter.features());
        println!("device supports");
        println!("{:?}", device.features());

        let sc_desc = SwapChainDescriptor {
            usage: TextureUsage::RENDER_ATTACHMENT,
            format: adapter.get_swap_chain_preferred_format(&surface),
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
        };

        let swapchain = device.create_swap_chain(&surface, &sc_desc);

        let depth_texture = texture::MyTexture::depth(&device, &sc_desc, "depth texture");

        let diffuse_bytes = include_bytes!("../happy-tree.bdff8a19.png");
        let diffuse_texture =
            texture::MyTexture::from_bytes(&device, &queue, diffuse_bytes, "happy tree").unwrap();

        let texture_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("diff tex bind group layuot"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStage::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStage::FRAGMENT,
                        ty: BindingType::Sampler {
                            comparison: false,
                            filtering: false,
                        },
                        count: None,
                    },
                ],
            });

        let diffuse_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("diff bind group"),
            layout: &texture_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&diffuse_texture.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
        });

        let instances = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(move |z| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let position = Vector3::new(x as f32, 0.0, z as f32) - INSTANCE_DISPLACEMENT;
                    let rotation = if position.is_zero() {
                        Quaternion::from_axis_angle(Vector3::unit_z(), Deg(0.0))
                    } else {
                        Quaternion::from_axis_angle(position.clone().normalize(), Deg(45.0))
                    };

                    MyInstance { position, rotation }
                })
            })
            .collect::<Vec<_>>();

        let instance_data = instances.iter().map(MyInstance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("instance buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: BufferUsage::VERTEX,
        });

        let camera = camera::Camera {
            eye: (0.0, 1.0, 2.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: Vector3::unit_y(),
            aspect: sc_desc.width as f32 / sc_desc.height as f32,
            fov: 45.0,
            z_near: 0.1,
            z_far: 100.0,
            speed: 0.2,
            inputs: camera::Inputs::default(),
        };

        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(camera.build_view_proj_matrix());

        let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("uniform buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("uniform bgl"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStage::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let uniform_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("uniform bg"),
            layout: &uniform_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let vs_module =
            device.create_shader_module(&wgpu::include_spirv!("../shaders/shader.vert.spv"));
        let fs_module =
            device.create_shader_module(&wgpu::include_spirv!("../shaders/shader.frag.spv"));

        let pipe_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pipe layout"),
            bind_group_layouts: &[&texture_bind_group_layout, &uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("pipe"),
            layout: Some(&pipe_layout),
            vertex: VertexState {
                module: &vs_module,
                entry_point: "main",
                buffers: &[Vertex::desc(), InstanceRawXd::desc()],
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
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: CullMode::Back,
                polygon_mode: PolygonMode::Fill,
            },
            depth_stencil: Some(DepthStencilState {
                format: texture::MyTexture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
                clamp_depth: false,
            }),
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        });

        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("pentagon vertex buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: BufferUsage::VERTEX,
        });

        let index_buffer_pentagon = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("pentagon index buffer"),
            contents: bytemuck::cast_slice(INDICES_PENTAGON),
            usage: BufferUsage::INDEX,
        });

        let num_indices_pentagon = INDICES_PENTAGON.len() as u32;

        Self {
            surface,
            device,
            queue,
            sc_desc,
            swapchain,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer_pentagon,
            num_indices_pentagon,
            diffuse_bind_group,
            diffuse_texture,
            depth_texture,
            camera,
            uniforms,
            uniform_buffer,
            uniform_bind_group,
            instances,
            instance_buffer,
        }
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swapchain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
        self.depth_texture =
            texture::MyTexture::depth(&self.device, &self.sc_desc, "depth texture resized");
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        self.camera.process_input(event)
    }

    fn update(&mut self) {
        self.camera.update();
        self.uniforms
            .update_view_proj(self.camera.build_view_proj_matrix());
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );
    }

    fn render(&mut self) -> Result<(), SwapChainError> {
        let frame = self.swapchain.get_current_frame()?.output;
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("state encoder"),
            });

        {
            let mut rp = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("state render pass"),
                color_attachments: &[RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &self.depth_texture.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            rp.set_pipeline(&self.render_pipeline);

            rp.set_bind_group(0, &self.diffuse_bind_group, &[]);
            rp.set_bind_group(1, &self.uniform_bind_group, &[]);

            rp.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rp.set_vertex_buffer(1, self.instance_buffer.slice(..));
            rp.set_index_buffer(self.index_buffer_pentagon.slice(..), IndexFormat::Uint16);

            rp.draw_indexed(
                0..self.num_indices_pentagon,
                0,
                0..self.instances.len() as _,
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut state = futures::executor::block_on(State::new(&window));

    event_loop.run(move |event, _, control_flow| {
        //*control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() && !state.input(event) => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput { input, .. } => match input {
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    _ => {}
                },
                WindowEvent::Resized(size) => {
                    state.resize(*size);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    state.resize(**new_inner_size);
                }
                _ => {}
            },
            Event::RedrawRequested(_) => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    Err(SwapChainError::Lost) => state.resize(state.size),
                    Err(SwapChainError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            _ => {}
        }
    });
}
