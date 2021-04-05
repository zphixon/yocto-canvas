#version 450

layout(location=0) in vec3 a_position;
layout(location=1) in vec3 a_normal;
layout(location=2) in vec2 a_tex_coords;

layout(location=5) in vec4 model_matrix_0;
layout(location=6) in vec4 model_matrix_1;
layout(location=7) in vec4 model_matrix_2;
layout(location=8) in vec4 model_matrix_3;

layout(location=0) out vec3 v_normal;
layout(location=1) out vec2 v_tex_coords;

layout(set=1, binding=0) uniform Uniforms {
    mat4 u_view_proj;
};

void main() {
    mat4 model = mat4(
        model_matrix_0,
        model_matrix_1,
        model_matrix_2,
        model_matrix_3
    );

    v_tex_coords = vec2(a_tex_coords.x, 1-a_tex_coords.y);
    v_normal = a_normal;
    gl_Position = u_view_proj * model * vec4(a_position, 1.0);
}
