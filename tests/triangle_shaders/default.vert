#version 460

layout(location = 0) in vec2 v_pos;
layout(location = 1) in vec3 v_color;

layout(location = 0) out vec3 frag_color;

void main() {
    frag_color = v_color;
    gl_Position = vec4(v_pos, 0.0, 1.0);
}