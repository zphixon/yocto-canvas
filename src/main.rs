use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

// i don't want to type 'wgpu' a thousand times thank you very much
use wgpu::{
    BackendBit, BlendState, Color, ColorTargetState, ColorWrite, CommandEncoderDescriptor,
    CullMode, Device, DeviceDescriptor, Features, FragmentState, FrontFace, Instance, Limits,
    LoadOp, MultisampleState, Operations, PipelineLayoutDescriptor, PolygonMode, PowerPreference,
    PresentMode, PrimitiveState, PrimitiveTopology, Queue, RenderPassColorAttachmentDescriptor,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions,
    ShaderFlags, ShaderModuleDescriptor, Surface, SwapChain, SwapChainDescriptor, SwapChainError,
    TextureUsage, VertexState,
};

struct State {
    surface: Surface,
    device: Device,
    queue: Queue,
    sc_desc: SwapChainDescriptor,
    swapchain: SwapChain,
    size: PhysicalSize<u32>,
    render_pipeline: RenderPipeline,
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

        let vs_data = wgpu::util::make_spirv(include_bytes!("../shaders/shader.vert.spv"));
        let fs_data = wgpu::util::make_spirv(include_bytes!("../shaders/shader.frag.spv"));

        let vs_module = device.create_shader_module(&ShaderModuleDescriptor {
            label: Some("vertex"),
            source: vs_data,
            flags: ShaderFlags::default(),
        });

        let fs_module = device.create_shader_module(&ShaderModuleDescriptor {
            label: Some("vertex"),
            source: fs_data,
            flags: ShaderFlags::default(),
        });

        let pipe_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pipe layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("pipe"),
            layout: Some(&pipe_layout),
            vertex: VertexState {
                module: &vs_module,
                entry_point: "main",
                buffers: &[],
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
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        });

        Self {
            surface,
            device,
            queue,
            sc_desc,
            swapchain,
            size,
            render_pipeline,
        }
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swapchain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            _ => false,
        }
    }

    fn update(&mut self) {}

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
                depth_stencil_attachment: None,
            });

            rp.set_pipeline(&self.render_pipeline);
            rp.draw(0..3, 0..1);
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
        *control_flow = ControlFlow::Wait;
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
