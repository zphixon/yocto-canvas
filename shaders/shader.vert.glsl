#version 450

layout(location=0) in vec2 a_position;
layout(location=1) in vec2 a_tex_coords;

layout(location=0) out vec2 v_tex_coords;

layout(set=1, binding=0) uniform Uniform {
    mat4 model;
    mat4 view;
    float zoom; // 😭
};

void main() {
    // TODO determine if necessary
    //v_tex_coords = vec2(a_tex_coords.x, 1 - a_tex_coords.y);
    v_tex_coords = a_tex_coords;
    vec2 pos = zoom * a_position;
    gl_Position = view * model * vec4(pos, 0.0, 1.0);
}
