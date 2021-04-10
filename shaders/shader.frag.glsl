#version 450

layout(location=0) out vec4 f_color;

//layout(set=0, binding=0) uniform texture2D t_diffuse;
//layout(set=0, binding=1) uniform sampler s_diffuse;

void main() {
    f_color = vec4(0.3, 0.95, 0.12, 1.0);
}
