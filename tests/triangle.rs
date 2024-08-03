/// Simple triangle with different colored vertices

use std::{mem::size_of, ptr::null};

use ash::vk;
use glfw::fail_on_errors;
use vust::{buffer::Buffer, create_info::VustCreateInfo, DrawCall, Vust};
use winapi::um::libloaderapi::GetModuleHandleW;

#[test]
fn triangle() {
    let mut glfw = glfw::init(glfw::fail_on_errors!()).unwrap();
    glfw.window_hint(glfw::WindowHint::Resizable(false));
    glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));

    let (window, _) = glfw.create_window(800, 600, "Vust Triangle Test", glfw::WindowMode::Windowed).unwrap();

    let vust_create_info = VustCreateInfo::default()
        .with_app_name("Vust Triangle Test")
        .with_app_version(vust::make_api_version(0, 0, 1, 0))
        .with_extensions(glfw.get_required_instance_extensions().unwrap())
        .with_surface_create_info(
            vust::create_info::SurfaceCreateInfo::Win32 {
                hinstance: unsafe { GetModuleHandleW(null()).cast() },
                hwnd: window.get_win32_window()
            }
        )
        .with_framebuffer_size((window.get_framebuffer_size().0 as usize, window.get_framebuffer_size().1 as usize))
        .with_graphics_pipeline_create_infos(vec![
            vust::create_info::GraphicsPipelineCreateInfo {
                name: "triangle pipeline".to_string(),
                vertex_bin: include_bytes!("triangle_shaders/default.vert.spv").to_vec(),
                fragment_bin: include_bytes!("triangle_shaders/default.frag.spv").to_vec(),
                vertex_binding_descriptions: vec![
                    vk::VertexInputBindingDescription::builder()
                        .binding(0)
                        .stride((size_of::<f32>() * 5) as u32)
                        .input_rate(vk::VertexInputRate::VERTEX)
                        .build()
                ],
                vertex_attribute_descriptions: vec![
                    vk::VertexInputAttributeDescription::builder()
                        .binding(0)
                        .location(0)
                        .offset(0)
                        .format(vk::Format::R32G32_SFLOAT)
                        .build(),
                    vk::VertexInputAttributeDescription::builder()
                        .binding(0)
                        .location(1)
                        .offset(8)
                        .format(vk::Format::R32G32B32_SFLOAT)
                        .build()
                ],
                topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                viewport: vust::create_info::Viewport::Dynamic,
                scissor: vust::create_info::Scissor::Dynamic,
                polygon_mode: vk::PolygonMode::FILL,
                cull_mode: vust::create_info::CullMode::None,
                descriptor_set_layouts: vec![]
            }
        ]);

    let mut vust = Vust::new(vust_create_info);

    let triangle_buffer = Buffer::builder()
        .with_name("Triangle Buffer")
        .with_usage(vk::BufferUsageFlags::VERTEX_BUFFER)
        .with_memory_location(vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT)
        .with_data(&[
            -0.5f32, -0.5, 1.0, 0.0, 0.0,
            0.5, -0.5, 0.0, 1.0, 0.0,
            0.0, 0.5, 0.0, 0.0, 1.0
        ])
        .build(&mut vust, true);

    while !window.should_close() {
        glfw.poll_events();

        vust.reset_command_buffer();
        vust.draw(DrawCall {
            graphics_pipeline: "triangle pipeline".to_string(),
            vertex_buffer: triangle_buffer.handle(),
            vertex_count: 3,
            viewport: Some(vk::Viewport { x: 0.0, y: 0.0, width: 800.0, height: 600.0, min_depth: 0.0, max_depth: 1.0 }),
            scissor: Some(vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width: 800, height: 600 } }),
            vertex_buffer_offset: 0
        });
        vust.render_surface();
    }
}