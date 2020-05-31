#version 450

layout(binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 proj;
}
ubo;

layout(location = 0) in vec3 position;
layout(location = 1) in vec2 texture_coords;

layout(location = 0) out vec2 fragTexCoord;

void main() {
    gl_Position = ubo.proj * ubo.view * ubo.model * vec4(position, 1.0);
    fragTexCoord = texture_coords;
}
