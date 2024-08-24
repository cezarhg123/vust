#version 460

layout(location = 0) in vec2 v_pos;
layout(location = 1) in vec2 v_uv;

layout(location = 0) out vec2 frag_uv;

void main() {
    frag_uv = v_uv;
    gl_Position = vec4(v_pos, 0.0, 1.0);
}
