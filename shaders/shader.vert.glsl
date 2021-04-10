#version 450

layout(location=0) in vec2 a_position;
layout(location=1) in vec2 a_tex_coords;

layout(location=0) out vec2 v_tex_coords;

void main() {
    v_tex_coords = vec2(a_tex_coords.x, 1 - a_tex_coords.y);
    gl_Position = vec4(a_position, 0.0, 1.0);
}
