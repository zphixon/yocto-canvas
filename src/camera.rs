use cgmath::{Deg, Matrix4, Point3, Vector3};
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[derive(Default)]
pub struct Inputs {
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
}

pub struct Camera {
    pub eye: Point3<f32>,
    pub target: Point3<f32>,
    pub up: Vector3<f32>,
    pub aspect: f32,
    pub fov: f32,
    pub z_near: f32,
    pub z_far: f32,
    pub speed: f32,
    pub inputs: Inputs,
}

impl Camera {
    pub fn build_view_proj_matrix(&self) -> Matrix4<f32> {
        let view = Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::perspective(Deg(self.fov), self.aspect, self.z_near, self.z_far);
        OPENGL_TO_WGPU_MATRIX * proj * view
    }

    pub fn process_input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput { input: KeyboardInput { state, virtual_keycode: Some(key), .. }, .. } => {
                let state = state == &ElementState::Pressed;
                if key == &VirtualKeyCode::W {
                    self.inputs.forward = state;
                    true
                } else if key == &VirtualKeyCode::S {
                    self.inputs.backward = state;
                    true
                } else if key == &VirtualKeyCode::A {
                    self.inputs.left = state;
                    true
                } else if key == &VirtualKeyCode::D {
                    self.inputs.right = state;
                    true
                } else {
                    false
                }
            },
            _ => false,
        }
    }

    pub fn update(&mut self) {
        use cgmath::InnerSpace;
        let forward = self.target - self.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        // Prevents glitching when self gets too close to the
        // center of the scene.
        if self.inputs.forward && forward_mag > self.speed {
            self.eye += forward_norm * self.speed;
        }
        if self.inputs.backward {
            self.eye -= forward_norm * self.speed;
        }

        let right = forward_norm.cross(self.up);

        // Redo radius calc in case the up/ down is pressed.
        let forward = self.target - self.eye;
        let forward_mag = forward.magnitude();

        if self.inputs.right {
            // Rescale the distance between the target and eye so
            // that it doesn't change. The eye therefore still
            // lies on the circle made by the target and eye.
            self.eye = self.target - (forward + right * self.speed).normalize() * forward_mag;
        }
        if self.inputs.left {
            self.eye = self.target - (forward - right * self.speed).normalize() * forward_mag;
        }
    }
}
