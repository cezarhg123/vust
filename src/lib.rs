pub mod create_info;

// expose a few ash/vk things
pub use ash::vk::{make_api_version, VertexInputBindingDescription, VertexInputAttributeDescription, PrimitiveTopology};
use std::{collections::HashMap, ffi::{CStr, CString}};
use ash::{extensions, vk};
use create_info::{CullMode, Scissor, Viewport, VustCreateInfo};

pub struct Vust {
    entry: ash::Entry,
    instance: ash::Instance,

    #[cfg(debug_assertions)]
    debug_utils_loader: extensions::ext::DebugUtils,
    #[cfg(debug_assertions)]
    debug_utils_messenger: vk::DebugUtilsMessengerEXT,

    physical_device: vk::PhysicalDevice,

    device: ash::Device,
    queue_index: u32,
    queue: vk::Queue,

    surface_util: extensions::khr::Surface,
    surface: vk::SurfaceKHR,

    swapchain_format: vk::SurfaceFormatKHR,
    extent: vk::Extent2D,
    swapchain: vk::SwapchainKHR,
    swapchain_image_views: Vec<vk::ImageView>,

    command_pool: vk::CommandPool,

    depth_image: vk::Image,
    depth_image_view: vk::ImageView,
    depth_image_memory: vk::DeviceMemory,

    depth_renderpass: vk::RenderPass,
    no_depth_renderpass: vk::RenderPass,
    
    graphics_pipelines: HashMap<String, vk::Pipeline>,
    swapchain_framebuffers: Vec<vk::Framebuffer>
}

impl Vust {
    pub const NAME: &'static str = "Vust";
    pub const C_NAME: &'static CStr = unsafe {
        CStr::from_bytes_with_nul_unchecked(b"Vust\0")
    };
    pub const VERSION: u32 = vk::make_api_version(0, 0, 1, 0);


    pub fn new(mut create_info: VustCreateInfo) -> Self {
        unsafe {
            let entry = ash::Entry::load().unwrap();
            #[cfg(debug_assertions)]
            println!("Loaded ash entry");

            let instance = {
                let app_info = vk::ApplicationInfo::builder()
                    .application_name(&create_info.app_name)
                    .application_version(create_info.app_version)
                    .engine_name(Vust::C_NAME)
                    .engine_version(Vust::VERSION)
                    .api_version(vk::make_api_version(0, 1, 3, 0))
                    .build();

                #[cfg(debug_assertions)] {
                    // only enable debug utils in debug build
                    create_info.enabled_instance_extensions.push(CString::new("VK_EXT_debug_utils").unwrap());
                    
                    println!("enabled instance extensions: ");
                    for ext in &create_info.enabled_instance_extensions {
                        println!("\t{}", ext.to_str().unwrap());
                    }
                }
                let enabled_instance_extension_ptrs = create_info.enabled_instance_extensions.iter().map(|ext| ext.as_ptr()).collect::<Vec<_>>();

                let enabled_layers = [CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
                let enabled_layer_ptrs = enabled_layers.iter().map(|layer| layer.as_ptr()).collect::<Vec<_>>();

                let instance_info = vk::InstanceCreateInfo::builder()
                    .application_info(&app_info)
                    .enabled_extension_names(&enabled_instance_extension_ptrs)
                    .enabled_layer_names(&enabled_layer_ptrs)
                    .build();

                entry.create_instance(&instance_info, None).unwrap()
            };
            #[cfg(debug_assertions)]
            println!("created vulkan instance");

            #[cfg(debug_assertions)]
            let (debug_utils_loader, debug_utils_messenger) = {
                let debug_utils_loader = extensions::ext::DebugUtils::new(&entry, &instance);
                let debug_utils_messenger = debug_utils_loader
                    .create_debug_utils_messenger(&vk::DebugUtilsMessengerCreateInfoEXT::builder()
                    .message_severity(
                        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR |
                        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING |
                        vk::DebugUtilsMessageSeverityFlagsEXT::INFO |
                        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    )
                    .message_type(
                        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL |
                        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE |
                        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    )
                    .pfn_user_callback(Some(Vust::vulkan_debug_callback))
                    .build(), None)
                    .unwrap();

                println!("created vulkan debug utils messenger");
                (debug_utils_loader, debug_utils_messenger)
            };

            let physical_devices = instance.enumerate_physical_devices().unwrap();
            let physical_device = physical_devices.into_iter().find(|physical_device| {
                let properties = instance.get_physical_device_properties(*physical_device);

                let physical_device_info = create_info::PhysicalDevice {
                    name: CStr::from_ptr(properties.device_name.as_ptr()).to_str().unwrap().to_string(),
                    device_type: match properties.device_type {
                        vk::PhysicalDeviceType::DISCRETE_GPU => create_info::PhysicalDeviceType::Discrete,
                        vk::PhysicalDeviceType::INTEGRATED_GPU => create_info::PhysicalDeviceType::Integrated,
                        _ => create_info::PhysicalDeviceType::NotSupported
                    }
                };

                (create_info.choose_physical_device)(physical_device_info)
            }).unwrap();

            #[cfg(debug_assertions)]
            println!("using physical device: {}", CStr::from_ptr(instance.get_physical_device_properties(physical_device).device_name.as_ptr()).to_str().unwrap());

            let (device, queue_index, queue) = {
                let queue_families = instance.get_physical_device_queue_family_properties(physical_device);
                let graphics_queue_family = queue_families
                    .into_iter()
                    .enumerate()
                    .find(|(_, p)| p.queue_flags.contains(vk::QueueFlags::GRAPHICS))
                    .unwrap();
    
                let enabled_device_extensions = [
                    CString::new("VK_KHR_swapchain").unwrap()
                ];
                let enabled_device_extension_ptrs = enabled_device_extensions.iter().map(|ext| ext.as_ptr()).collect::<Vec<_>>();
    
                let queue_create_infos = vec![
                    vk::DeviceQueueCreateInfo::builder()
                        .queue_family_index(graphics_queue_family.0 as u32)
                        .queue_priorities(&[1.0])
                        .build()
                ];

                let physical_device_features = instance.get_physical_device_features(physical_device);

                let device = instance.create_device(
                    physical_device,
                    &vk::DeviceCreateInfo::builder()
                        .queue_create_infos(&queue_create_infos)
                        .enabled_extension_names(&enabled_device_extension_ptrs)
                        .enabled_features(&physical_device_features)
                        .build(),
                    None
                ).unwrap();

                let queue = device.get_device_queue(graphics_queue_family.0 as u32, 0);

                (device, graphics_queue_family.0 as u32, queue)
            };
            #[cfg(debug_assertions)]
            println!("created vulkan logical device");

            let surface_util = extensions::khr::Surface::new(&entry, &instance);

            let surface;

            #[cfg(target_os = "windows")] {
                let (hinstance, hwnd) = create_info.surface_create_info.into_win32();
                let win32_surface_util = extensions::khr::Win32Surface::new(&entry, &instance);

                surface = win32_surface_util.create_win32_surface(
                    &vk::Win32SurfaceCreateInfoKHR::builder()
                        .hinstance(hinstance)
                        .hwnd(hwnd)
                        .build(),
                    None
                ).unwrap();

                println!("created win32 vulkan surface");
            }

            let swapchain_util = extensions::khr::Swapchain::new(&instance, &device);

            let capabilities = surface_util.get_physical_device_surface_capabilities(physical_device, surface).unwrap();
            let swapchain_format = surface_util.get_physical_device_surface_formats(physical_device, surface).unwrap().into_iter()
                .find(|format| format.format == vk::Format::B8G8R8A8_SRGB && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR)
                .unwrap();

            let framebuffer = create_info.framebuffer_size;

            let extent = vk::Extent2D {
                width: (framebuffer.0 as u32).clamp(capabilities.min_image_extent.width, capabilities.max_image_extent.width),
                height: (framebuffer.1 as u32).clamp(capabilities.min_image_extent.height, capabilities.max_image_extent.height)
            };

            let swapchain = swapchain_util.create_swapchain(
                &vk::SwapchainCreateInfoKHR::builder()
                    .surface(surface)
                    .min_image_count(capabilities.min_image_count + 1)
                    .image_format(swapchain_format.format)
                    .image_color_space(swapchain_format.color_space)
                    .image_extent(extent)
                    .image_array_layers(1)
                    .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                    .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .present_mode(vk::PresentModeKHR::IMMEDIATE)
                    .pre_transform(capabilities.current_transform)
                    .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                    .clipped(true)
                    .old_swapchain(vk::SwapchainKHR::null())
                    .build(),
                None
            ).unwrap();
            #[cfg(debug_assertions)]
            println!("created vulkan swapchain");

            let images = swapchain_util.get_swapchain_images(swapchain).unwrap();

            let swapchain_image_views = images.iter().map(|image| {
                device.create_image_view(
                    &vk::ImageViewCreateInfo::builder()
                        .image(*image)
                        .view_type(vk::ImageViewType::TYPE_2D)
                        .format(swapchain_format.format)
                        .components(vk::ComponentMapping {
                            r: vk::ComponentSwizzle::IDENTITY,
                            g: vk::ComponentSwizzle::IDENTITY,
                            b: vk::ComponentSwizzle::IDENTITY,
                            a: vk::ComponentSwizzle::IDENTITY
                        })
                        .subresource_range(vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            base_mip_level: 0,
                            level_count: 1,
                            base_array_layer: 0,
                            layer_count: 1
                        })
                        .build(),
                    None
                ).unwrap()
            }).collect::<Vec<_>>();
            #[cfg(debug_assertions)]
            println!("created vulkan swapchain image views");

            let command_pool = device.create_command_pool(
                &vk::CommandPoolCreateInfo::builder()
                    .queue_family_index(queue_index)
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                    .build(),
                None
            ).unwrap();
            #[cfg(debug_assertions)]
            println!("created vulkan command pool");
            
            // 2 renderpasses
            // 1 with depth testing
            // 1 without depth testing
            // im doing this so when i need to create say 2 graphic pipelines for UI and 3 for 3D i can just select which renderpass to use
            // tbh im not smart enough to understand how renderpasses work but we ball

            let color_attachment = vk::AttachmentDescription::builder()
                .format(swapchain_format.format)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .build();

            let color_attachment_ref = vk::AttachmentReference::builder()
                .attachment(0)
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .build();

            let no_depth_renderpass = device.create_render_pass(
                &vk::RenderPassCreateInfo::builder()
                    .attachments(&[color_attachment])
                    .subpasses(&[
                        vk::SubpassDescription::builder()
                            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                            .color_attachments(&[color_attachment_ref])
                            .build()
                    ])
                    .dependencies(&[
                        vk::SubpassDependency::builder()
                            .src_subpass(vk::SUBPASS_EXTERNAL)
                            .dst_subpass(0)
                            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                            .src_access_mask(vk::AccessFlags::empty())
                            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                            .build()
                    ]),
                None
            ).unwrap();

            let depth_format = {
                let wanted_formats = [vk::Format::D24_UNORM_S8_UINT, vk::Format::D32_SFLOAT_S8_UINT];
                
                let mut return_format = None;
                
                for format in wanted_formats {
                    let props = instance.get_physical_device_format_properties(physical_device, format);

                    if props.optimal_tiling_features.contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT) {
                        return_format = Some(format);
                    }
                }

                return_format.expect("physical device has no supported depth format")
            };

            let depth_attachment = vk::AttachmentDescription::builder()
                .format(depth_format)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::DONT_CARE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                .build();

            let depth_attachment_ref = vk::AttachmentReference::builder()
                .attachment(1)
                .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                .build();

            let depth_image = device.create_image(
                &vk::ImageCreateInfo::builder()
                    .image_type(vk::ImageType::TYPE_2D)
                    .extent(vk::Extent3D {
                        width: extent.width,
                        height: extent.height,
                        depth: 1,
                    })
                    .mip_levels(1)
                    .array_layers(1)
                    .format(depth_format)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .tiling(vk::ImageTiling::OPTIMAL)
                    .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .initial_layout(vk::ImageLayout::UNDEFINED)
                    .build(),
                None
            ).unwrap();

            let depth_image_memory = {
                let memory_requirements = device.get_image_memory_requirements(depth_image);
        
                device.allocate_memory(
                    &vk::MemoryAllocateInfo::builder()
                        .allocation_size(memory_requirements.size)
                        .memory_type_index(Self::find_memory_type(instance.get_physical_device_memory_properties(physical_device), memory_requirements.memory_type_bits, vk::MemoryPropertyFlags::DEVICE_LOCAL).unwrap())
                        .build(),
                    None
                ).unwrap()
            };

            device.bind_image_memory(depth_image, depth_image_memory, 0).unwrap();

            let depth_image_view = device.create_image_view(
                &vk::ImageViewCreateInfo::builder()
                    .image(depth_image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(depth_format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::IDENTITY,
                        g: vk::ComponentSwizzle::IDENTITY,
                        b: vk::ComponentSwizzle::IDENTITY,
                        a: vk::ComponentSwizzle::IDENTITY
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::DEPTH,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1
                    })
                    .build(),
                None
            ).unwrap();

            let transition_depth_image_command_buffer = device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_pool(command_pool)
                    .command_buffer_count(1)
                    .level(vk::CommandBufferLevel::PRIMARY)
            ).unwrap()[0];

            device.begin_command_buffer(
                transition_depth_image_command_buffer,
                &vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
                    .build(),
            ).unwrap();

            let barrier = vk::ImageMemoryBarrier::builder()
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(depth_image)
                .dst_access_mask(vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)
                .subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1)
                        .build()
                )
            .build();

            device.cmd_pipeline_barrier(
                transition_depth_image_command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier]
            );

            device.end_command_buffer(transition_depth_image_command_buffer).unwrap();

            device.queue_submit(
                queue,
                &[
                    vk::SubmitInfo::builder()
                        .command_buffers(&[transition_depth_image_command_buffer])
                        .build(),
                ],
                vk::Fence::null()
            ).unwrap();

            device.queue_wait_idle(queue).unwrap();

            device.free_command_buffers(
                command_pool,
                &[transition_depth_image_command_buffer],
            );

            let depth_renderpass = device.create_render_pass(
                &vk::RenderPassCreateInfo::builder()
                    .attachments(&[color_attachment, depth_attachment])
                    .subpasses(&[
                        vk::SubpassDescription::builder()
                            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                            .color_attachments(&[color_attachment_ref])
                            .depth_stencil_attachment(&depth_attachment_ref)
                            .build()
                    ])
                    .dependencies(&[
                        vk::SubpassDependency::builder()
                            .src_subpass(vk::SUBPASS_EXTERNAL)
                            .dst_subpass(0)
                            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS)
                            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS)
                            .src_access_mask(vk::AccessFlags::empty())
                            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)
                            .build()
                    ]),
                None
            ).unwrap();

            let mut graphics_pipelines = HashMap::new();

            for info in create_info.graphics_pipeline_create_infos.iter() {
                let vertex_input_state = device.create_shader_module(&vk::ShaderModuleCreateInfo {
                    s_type: vk::StructureType::SHADER_MODULE_CREATE_INFO,
                    code_size: info.vertex_bin.len(),
                    p_code: info.vertex_bin.as_ptr() as *const u32,
                    ..Default::default()
                }, None).unwrap();

                let fragment_input_state = device.create_shader_module(&vk::ShaderModuleCreateInfo {
                    s_type: vk::StructureType::SHADER_MODULE_CREATE_INFO,
                    code_size: info.fragment_bin.len(),
                    p_code: info.fragment_bin.as_ptr() as *const u32,
                    ..Default::default()
                }, None).unwrap();

                let entry_point_name = CString::new("main").unwrap();

                let shader_stages = [
                    vk::PipelineShaderStageCreateInfo::builder()
                        .stage(vk::ShaderStageFlags::VERTEX)
                        .module(vertex_input_state)
                        .name(&entry_point_name)
                        .build(),
                    vk::PipelineShaderStageCreateInfo::builder()
                        .stage(vk::ShaderStageFlags::FRAGMENT)
                        .module(fragment_input_state)
                        .name(&entry_point_name)
                        .build()
                ];

                let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
                    .vertex_binding_descriptions(&info.vertex_binding_descriptions)
                    .vertex_attribute_descriptions(&info.vertex_attribute_descriptions)
                    .build();

                let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
                    .topology(info.topology)
                    .primitive_restart_enable(false)
                    .build();

                // absolute unit of a match statement
                let viewport_state_info = match (info.viewport.clone(), info.scissor.clone()) {
                    (Viewport::Dynamic, Scissor::Dynamic) => {
                        vk::PipelineViewportStateCreateInfo::builder().viewport_count(1).scissor_count(1).build()
                    }
                    (Viewport::Dynamic, Scissor::Static { x, y, width, height }) => {
                        vk::PipelineViewportStateCreateInfo::builder()
                            .viewport_count(1)
                            .scissors(&[
                                vk::Rect2D {
                                    offset: vk::Offset2D { x, y },
                                    extent: vk::Extent2D { width, height }
                                }
                            ])
                            .build()
                    }
                    (Viewport::Static { x, y, width, height, min_depth, max_depth }, Scissor::Dynamic) => {
                        vk::PipelineViewportStateCreateInfo::builder()
                            .viewports(&[
                                vk::Viewport {
                                    x,
                                    y,
                                    width,
                                    height,
                                    min_depth,
                                    max_depth
                                }
                            ])
                            .scissor_count(1)
                            .build()
                    }
                    (Viewport::Static { x: v_x, y: v_y, width: v_width, height: v_height, min_depth: v_min_depth, max_depth: v_max_depth }, Scissor::Static { x: s_x, y: s_y, width: s_width, height: s_height }) => {
                        vk::PipelineViewportStateCreateInfo::builder()
                            .viewports(&[
                                vk::Viewport {
                                    x: v_x,
                                    y: v_y,
                                    width: v_width,
                                    height: v_height,
                                    min_depth: v_min_depth,
                                    max_depth: v_max_depth
                                }
                            ]).scissors(&[
                                vk::Rect2D {
                                    offset: vk::Offset2D { x: s_x, y: s_y },
                                    extent: vk::Extent2D { width: s_width, height: s_height }
                                }
                            ])
                            .build()
                    }
                };

                let rasterizer_info = vk::PipelineRasterizationStateCreateInfo::builder()
                    .depth_clamp_enable(false)
                    .rasterizer_discard_enable(false)
                    .polygon_mode(info.polygon_mode)
                    .line_width(1.0)
                    .cull_mode(if let CullMode::None = info.cull_mode { vk::CullModeFlags::NONE } else { vk::CullModeFlags::BACK })
                    .front_face(
                        match &info.cull_mode {
                            CullMode::None => vk::FrontFace::CLOCKWISE, // doesnt matter cuz cull mode is none
                            CullMode::Clockwise => vk::FrontFace::CLOCKWISE,
                            CullMode::AntiClockwise => vk::FrontFace::COUNTER_CLOCKWISE
                        }
                    )
                    .depth_bias_enable(false)
                    .build();

                let multisample_info = vk::PipelineMultisampleStateCreateInfo::builder()
                    .rasterization_samples(vk::SampleCountFlags::TYPE_1)
                    .sample_shading_enable(false)
                    .build();

                let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
                    .color_write_mask(vk::ColorComponentFlags::RGBA)
                    .blend_enable(true)
                    .color_blend_op(vk::BlendOp::ADD)
                    .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                    .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                    .src_alpha_blend_factor(vk::BlendFactor::ONE)
                    .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                    .alpha_blend_op(vk::BlendOp::ADD)
                    .build();

                let color_blend_info = vk::PipelineColorBlendStateCreateInfo::builder()
                    .attachments(&[color_blend_attachment])
                    .logic_op_enable(true)
                    .logic_op(vk::LogicOp::COPY)
                    .blend_constants([0.0, 0.0, 0.0, 0.0])
                    .build();

                let descriptor_set_layouts = info.descriptor_set_layouts.iter().map(|descriptor_set_layout| {
                    let bindings = descriptor_set_layout.bindings.iter().enumerate().map(|(i, descriptor_set_binding)| {
                        vk::DescriptorSetLayoutBinding::builder()
                            .binding(i as u32)
                            .descriptor_type(descriptor_set_binding.descriptor_type)
                            .descriptor_count(1)
                            .stage_flags(descriptor_set_binding.stage_flags)
                            .build()
                    }).collect::<Vec<_>>();
                    
                    device.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings).build(), None).unwrap()
                }).collect::<Vec<_>>();

                let pipeline_layout = device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo::builder().set_layouts(&descriptor_set_layouts).build(), None).unwrap();
                
                let mut dynamic_states = Vec::new();
                
                if let Viewport::Dynamic = info.viewport {
                    dynamic_states.push(vk::DynamicState::VIEWPORT);
                }
                if let Scissor::Dynamic = info.scissor {
                    dynamic_states.push(vk::DynamicState::SCISSOR);
                }

                let dynamic_state_info = vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states).build();

                let depth_stencil_info = if info.enable_depth_test {
                    vk::PipelineDepthStencilStateCreateInfo::builder()
                        .depth_test_enable(true)
                        .depth_write_enable(true)
                        .depth_compare_op(vk::CompareOp::LESS)
                        .depth_bounds_test_enable(false)
                        .stencil_test_enable(false)
                        .build()
                } else {
                    vk::PipelineDepthStencilStateCreateInfo::builder()
                        .depth_test_enable(false)
                        .depth_write_enable(false)
                        .depth_bounds_test_enable(false)
                        .stencil_test_enable(false)
                        .build()
                };

                let pipeline = device.create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[
                        vk::GraphicsPipelineCreateInfo::builder()
                            .stages(&shader_stages)
                            .vertex_input_state(&vertex_input_info)
                            .input_assembly_state(&input_assembly_info)
                            .viewport_state(&viewport_state_info)
                            .rasterization_state(&rasterizer_info)
                            .multisample_state(&multisample_info)
                            .depth_stencil_state(&depth_stencil_info)
                            .color_blend_state(&color_blend_info)
                            .dynamic_state(&dynamic_state_info)
                            .layout(pipeline_layout)
                            .render_pass(if info.enable_depth_test { depth_renderpass } else { no_depth_renderpass })
                            .subpass(0)
                            .build()
                    ],
                    None
                ).unwrap()[0];

                graphics_pipelines.insert(info.name.clone(), pipeline);
            }
            #[cfg(debug_assertions)]
            println!("created {} graphics pipelines", graphics_pipelines.len());

            let swapchain_framebuffers = swapchain_image_views.iter().map(|image_view| {
                let attachments = [*image_view, depth_image_view];
                device.create_framebuffer(&vk::FramebufferCreateInfo::builder()
                    .render_pass(depth_renderpass)
                    .attachments(&attachments)
                    .width(extent.width)
                    .height(extent.height)
                    .layers(1)
                    .build(), None).unwrap()
            }).collect::<Vec<_>>();
            #[cfg(debug_assertions)]
            println!("created swapchain framebuffers");

            Self {
                entry,
                instance,

                #[cfg(debug_assertions)]
                debug_utils_loader,
                #[cfg(debug_assertions)]
                debug_utils_messenger,

                physical_device,

                device,
                queue_index,
                queue,

                surface_util,
                surface,

                swapchain_format,
                extent,
                swapchain,
                swapchain_image_views,

                command_pool,

                depth_image,
                depth_image_memory,
                depth_image_view,

                depth_renderpass,
                no_depth_renderpass,

                graphics_pipelines,
                swapchain_framebuffers
            }
        }
    }

    /// yoinked from ash examples
    unsafe extern "system" fn vulkan_debug_callback(
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
        message_type: vk::DebugUtilsMessageTypeFlagsEXT,
        p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
        _user_data: *mut std::os::raw::c_void,
    ) -> vk::Bool32 {
        let callback_data = *p_callback_data;
        let message_id_number = callback_data.message_id_number;

        let message_id_name = if callback_data.p_message_id_name.is_null() {
            std::borrow::Cow::from("")
        } else {
            std::ffi::CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
        };

        let message = if callback_data.p_message.is_null() {
            std::borrow::Cow::from("")
        } else {
            std::ffi::CStr::from_ptr(callback_data.p_message).to_string_lossy()
        };

        println!(
            "{message_severity:?}:\n{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n",
        );

        vk::FALSE
    }

    /// ported from https://vulkan-tutorial.com
    pub fn find_memory_type(
        memory_properties: vk::PhysicalDeviceMemoryProperties,
        type_filter: u32,
        properties: vk::MemoryPropertyFlags
    ) -> Option<u32> {
        for i in 0..memory_properties.memory_type_count {
            // dont really know how this works ¯\_(ツ)_/¯
            if (type_filter & (1 << i)) > 0 && ((memory_properties.memory_types[i as usize].property_flags & properties) == properties) {
                return Some(i as u32);
            }
        }

        None
    }
}
