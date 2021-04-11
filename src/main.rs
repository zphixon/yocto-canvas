pub use anyhow::{Context, Result};

use bytemuck::{Pod, Zeroable};

use cgmath::{Matrix4, Point3, SquareMatrix, Vector3};

use image::RgbaImage;

use texture::MyTexture;

use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BackendBit, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendState, Buffer, BufferAddress, BufferBindingType,
    BufferCopyView, BufferUsage, ColorTargetState, ColorWrite, CommandEncoderDescriptor, CullMode,
    Device, DeviceDescriptor, Extent3d, Features, FragmentState, FrontFace, InputStepMode,
    Instance, LoadOp, MultisampleState, Operations, Origin3d, PipelineLayoutDescriptor,
    PolygonMode, PresentMode, PrimitiveState, PrimitiveTopology, Queue,
    RenderPassColorAttachmentDescriptor, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, RequestAdapterOptions, ShaderStage, Surface, SwapChain,
    SwapChainDescriptor, SwapChainError, TextureCopyView, TextureDataLayout, TextureUsage,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexState,
};

use std::collections::HashMap;

mod texture;

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
    model: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
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
    pipeline: RenderPipeline,
    texture: MyTexture,
    image: RgbaImage,
    vertex_buffer: Buffer,
    // 😠 https://github.com/rust-windowing/winit/issues/883
    mouse: Mouse,
    updated_uniforms: bool,
    uniform_buffer: Buffer,
    uniform_bind_group: BindGroup,
}

impl State {
    async fn new(window: &Window) -> Result<Self> {
        let size = window.inner_size();
        let instance = Instance::new(BackendBit::PRIMARY);
        //let instance = Instance::new(BackendBit::DX12);
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

        let (texture, image) = MyTexture::empty(&device, &queue, "image")?;

        //#[rustfmt::skip]
        //let model = Matrix4::new(
        //    image.width() as f32 / size.width as f32, 0., 0., 0.,
        //    0., image.height() as f32 / size.height as f32, 0., 0.,
        //    0., 0., 1., 0.,
        //    0., 0., 0., 1.,
        //).into();

        let updated_uniforms = true;

        let uniform = Uniform {
            zoom: 1.0f32,
            model: Matrix4::identity().into(),
            view: Matrix4::identity().into(),
        };

        let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("uniform"),
            contents: bytemuck::cast_slice(&[uniform]),
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
            label: Some("uniform b group"),
            layout: &uniform_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pipeline layout"),
            bind_group_layouts: &[&texture.group_layout, &uniform_bind_group_layout],
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

        let mouse = Mouse {
            x: size.width as f32 / 2.,
            y: size.height as f32 / 2.,
            left: ElementState::Released,
            right: ElementState::Released,
        };

        Ok(Self {
            surface,
            device,
            queue,
            swapchain,
            sc_desc,
            size,
            pipeline,
            texture,
            image,
            vertex_buffer,
            mouse,
            updated_uniforms,
            uniform_buffer,
            uniform_bind_group,
        })
    }

    // returns true if state captured the event, false otherwise
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
            _ => false,
        }
    }

    fn update(&mut self) {
        if self.mouse.left == ElementState::Pressed {
            //println!("{}", 300_000 % 4);
            self.image.as_mut()[300_000] = 0xff;
            self.image.as_mut()[300_001] = 0xff;
            self.image.as_mut()[300_002] = 0xff;
        } else {
            self.image.as_mut()[300_000] = 0x0f;
            self.image.as_mut()[300_001] = 0x0f;
            self.image.as_mut()[300_002] = 0x0f;
        }

        if !self.updated_uniforms {
            //let view: [[f32; 4]; 4] = (
            //    //OPENGL_TO_WGPU_MATRIX *
            //    cgmath::ortho(
            //        0.0,
            //        self.size.width as f32,
            //        self.size.height as f32,
            //        0.0,
            //        0.0,
            //        1.0,
            //    )
            //)
            //.into();
            let view = Matrix4::identity().into();

            //#[rustfmt::skip]
            //let model = (OPENGL_TO_WGPU_MATRIX *
            //    Matrix4::new(
            //        self.image.width() as f32 / self.size.width as f32, 0., 0., 0.,
            //        0., self.image.height() as f32 / self.size.height as f32, 0., 0.,
            //        0., 0., 1., 0.,
            //        0., 0., 0., 1.,
            //    ))
            //    .into();
            let model = Matrix4::identity().into();

            //println!("{:?}", view);

            // this should work fine. however, for some reason the last row has been moved to the front in vulkan,
            // and in dx12, the entire uniform is zero. i have no idea why this is happening and it's quite frustrating.
            let uniform = Uniform {
                zoom: 1.0f32,
                model,
                view,
            };

            self.queue
                .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));
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

        self.queue.write_texture(
            TextureCopyView {
                texture: &self.texture.texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
            },
            self.image.as_raw(),
            self.texture.layout.clone(),
            self.texture.size.clone(),
        );

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

            rp.set_viewport(
                0.,
                0.,
                self.size.width as f32,
                self.size.height as f32,
                0.,
                1.,
            );

            rp.set_pipeline(&self.pipeline);

            rp.set_bind_group(0, &self.texture.group, &[]);
            rp.set_bind_group(1, &self.uniform_bind_group, &[]);

            rp.set_vertex_buffer(0, self.vertex_buffer.slice(..));

            let len = VERTICES.len() as u32;
            rp.draw(0..len, 0..1);
        }

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
        state.update();
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if state.input(&event) {
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
                        WindowEvent::Resized(size) => state.resize(*size),
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            state.resize(**new_inner_size);
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
