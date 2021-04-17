pub use anyhow::{Context, Result};

use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use wgpu::SwapChainError;

mod backend_wgpu;
mod composite;
mod image;
mod texture;

use crate::{backend_wgpu::WgpuBackend, image::Pixel};

#[derive(Debug)]
struct Mouse {
    x: f32,
    y: f32,
    left: ElementState,
    right: ElementState,
}

#[allow(dead_code)]
struct State {
    size: PhysicalSize<u32>,
    mouse: Mouse,
    zoom: f32,
    // *perhaps* eventually have my own cpu backend? not sure
    wgpu_backend: Option<WgpuBackend>,
    cpu_backend: Option<()>,
}

impl State {
    async fn new(window: &Window) -> Result<Self> {
        let size = window.inner_size();

        let mouse = Mouse {
            x: size.width as f32 / 2.,
            y: size.height as f32 / 2.,
            left: ElementState::Released,
            right: ElementState::Released,
        };

        let zoom = 1.0;

        let wgpu_backend = Some(WgpuBackend::new(window).await?);

        Ok(Self {
            size,
            mouse,
            zoom,
            wgpu_backend,
            cpu_backend: None,
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
        // backend-agnostic stuff that's done slightly differently goes here
        if let Some(wgpu_backend) = &mut self.wgpu_backend {
            if self.mouse.left == ElementState::Pressed {
                wgpu_backend.canvas_pipeline.canvas_image.set_pixel(
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
                wgpu_backend.canvas_pipeline.canvas_image.set_pixel(
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

            // and backend-specific stuff goes in these methods
            wgpu_backend.update(&self.size, self.zoom);
        }
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;
        if let Some(wgpu_backend) = &mut self.wgpu_backend {
            wgpu_backend.resize(new_size);
        }
    }

    fn render(&mut self) -> Result<()> {
        if let Some(wgpu_backend) = &mut self.wgpu_backend {
            wgpu_backend.render(&self.size)?;
        }

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
