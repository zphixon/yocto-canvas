#version 450

layout(location=0) in vec2 a_position;
layout(location=1) in vec2 a_tex_coords;

layout(location=0) out vec2 v_tex_coords;

layout(set=1, binding=0) uniform Uniform {
    float scale_x;
    float scale_y;
    float xform_x;
    float xform_y;
    float zoom;
};

void main() {
    v_tex_coords = a_tex_coords;

    vec2 pos = zoom * a_position;
    pos.x *= scale_x;
    pos.y *= scale_y;

    gl_Position = vec4(pos, 0.0, 1.0);
}
