#version 460

layout(location = 0) out vec4 out_color;

layout(location = 0) in vec2 frag_uv;

layout(binding = 0) uniform sampler2D tex;

void main() {
    out_color = texture(tex, frag_uv);
}
