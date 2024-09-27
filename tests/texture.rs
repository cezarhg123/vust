/// Rect with texture and index buffer

use std::{io::Cursor, mem::size_of, ptr::null};
use ash::vk;
use glfw::fail_on_errors;
use image::GenericImageView;
use vust::{buffer::Buffer, create_info::VustCreateInfo, pipeline::{DescriptorSetBinding, DescriptorSetLayout, GraphicsPipeline}, texture::Texture, write_descriptor_info::WriteDescriptorInfo, Vust};
use winapi::um::libloaderapi::GetModuleHandleW;

#[test]
fn texture() {
    let mut glfw = glfw::init(glfw::fail_on_errors!()).unwrap();
    glfw.window_hint(glfw::WindowHint::Resizable(false));
    glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));

    let (window, _) = glfw.create_window(800, 600, "Texture Test", glfw::WindowMode::Windowed).unwrap();

    let vust_create_info = VustCreateInfo::default()
        .with_app_name("Texture Test")
        .with_app_version(vust::make_api_version(0, 0, 1, 0))
        .with_extensions(glfw.get_required_instance_extensions().unwrap())
        .with_surface_create_info(
            vust::create_info::SurfaceCreateInfo::Win32 {
                hinstance: unsafe { GetModuleHandleW(null()).cast() },
                hwnd: window.get_win32_window()
            }
        )
        .with_framebuffer_size((window.get_framebuffer_size().0 as usize, window.get_framebuffer_size().1 as usize));

    let mut vust = Vust::new(vust_create_info);

    let pipeline = GraphicsPipeline::new(
        &vust,
        vust::pipeline::GraphicsPipelineCreateInfo {
            name: "texture pipeline".to_string(),
            vertex_bin: include_bytes!("texture_shaders/default.vert.spv").to_vec(),
            fragment_bin: include_bytes!("texture_shaders/default.frag.spv").to_vec(),
            vertex_binding_descriptions: vec![
                vk::VertexInputBindingDescription::builder()
                    .binding(0)
                    .stride((size_of::<f32>() * 4) as u32)
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
                    .format(vk::Format::R32G32_SFLOAT)
                    .build()
            ],
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            viewport: vust::pipeline::Viewport::Dynamic,
            scissor: vust::pipeline::Scissor::Dynamic,
            polygon_mode: vk::PolygonMode::FILL,
            cull_mode: vust::pipeline::CullMode::None,
            descriptor_set_layout: Some(
                DescriptorSetLayout {
                    bindings: vec![
                        DescriptorSetBinding {
                            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                            stage_flags: vk::ShaderStageFlags::FRAGMENT
                        }
                    ]
                }
            )
        }
    );

    let descriptor = pipeline.create_descriptor(&mut vust).unwrap();

    let rect_buffer = Buffer::builder()
        .with_name("Rect Vertex Buffer")
        .with_usage(vk::BufferUsageFlags::VERTEX_BUFFER)
        .with_memory_location(vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT)
        .with_data(&[
            -0.5f32, -0.5, 0.0, 0.0, // bottom left
            -0.5, 0.5, 0.0, 1.0, // top left
            0.5, 0.5, 1.0, 1.0, // top right
            0.5, -0.5, 1.0, 0.0 // bottom right
        ])
        .build(&mut vust, true);

    let index_buffer = Buffer::builder()
        .with_name("Rect Index Buffer")
        .with_usage(vk::BufferUsageFlags::INDEX_BUFFER)
        .with_memory_location(vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT)
        .with_data(&[0, 1, 2, 0, 2, 3])
        .build(&mut vust, true);

    let image = image::load(Cursor::new(include_bytes!("textures/green amogus.png")), image::ImageFormat::Png).unwrap();

    let texture = Texture::builder()
        .with_name("Texture Buffer")
        .with_data(image.as_bytes())
        .with_dimensions(image.dimensions())
        .with_format(vk::Format::R8G8B8A8_SRGB)
        .with_filter(vk::Filter::LINEAR)
        .build(&mut vust)
        .unwrap();

    while !window.should_close() {
        glfw.poll_events();

        vust.reset_command_buffer();
        vust.bind_pipeline(pipeline.handle());
        vust.bind_viewport(vk::Viewport { x: 0.0, y: 0.0, width: 800.0, height: 600.0, min_depth: 0.0, max_depth: 1.0 });
        vust.bind_scissor(vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width: 800, height: 600 } });
        vust.update_descriptor_set(&descriptor, &[WriteDescriptorInfo::Image { image_view: texture.view(), sampler: texture.sampler() }]);
        vust.bind_descriptor_set(&pipeline, &descriptor);
        vust.bind_vertex_buffer(rect_buffer.handle());
        vust.bind_index_buffer(index_buffer.handle());
        vust.draw_indexed(6);
        vust.render_surface();
    }

    vust.wait_idle();
}