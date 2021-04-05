#version 450

// -1 -1                1 -1
//
//
// -1  1                1  1

const vec2 corners[6] = vec2[6](
    vec2(-1.0, -1.0),
    vec2(1.0, -1.0),
    vec2(1.0, 1.0),

    vec2(1.0, 1.0),
    vec2(-1.0, 1.0),
    vec2(-1.0, -1.0)
);

void main() {
    gl_Position = vec4(corners[gl_VertexIndex], 0.0, 1.0);
}
