use std::{mem::size_of, ptr::null};

use ash::vk;
use glfw::fail_on_errors;
use vust::{create_info::VustCreateInfo, Vust};
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
                name: "Triangle".to_string(),
                vertex_bin: include_bytes!("triangle_shaders/default.vert.spv").to_vec(),
                fragment_bin: include_bytes!("triangle_shaders/default.frag.spv").to_vec(),
                enable_depth_test: false,
                vertex_binding_descriptions: vec![
                    vk::VertexInputBindingDescription::builder()
                        .binding(0)
                        .stride((size_of::<f32>() * 3) as u32 /* size of vec3 */)
                        .input_rate(vk::VertexInputRate::VERTEX)
                        .build()
                ],
                vertex_attribute_descriptions: vec![
                    vk::VertexInputAttributeDescription::builder()
                        .binding(0)
                        .location(0)
                        .offset(0)
                        .format(vk::Format::R32G32B32_SFLOAT)
                        .build()
                ],
                topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                viewport: vust::create_info::Viewport::Dynamic,
                scissor: vust::create_info::Scissor::Dynamic,
                polygon_mode: vk::PolygonMode::FILL,
                cull_mode: vust::create_info::CullMode::AntiClockwise,
                descriptor_set_layouts: vec![]
            }
        ]);

    let vust = Vust::new(vust_create_info);

    while !window.should_close() {
        glfw.poll_events();
    }
}