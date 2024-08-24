pub mod create_info;
pub mod buffer;
pub mod texture;
pub mod pipeline;
pub mod write_descriptor_info;

// expose a few ash/vk things
pub use ash::vk::{make_api_version, VertexInputBindingDescription, VertexInputAttributeDescription, PrimitiveTopology};
use create_info::VustCreateInfo;
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use pipeline::GraphicsPipeline;
use write_descriptor_info::WriteDescriptorInfo;
use std::{collections::HashMap, ffi::{CStr, CString}};
use ash::{extensions, vk};

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
    swapchain_util: extensions::khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    swapchain_image_views: Vec<vk::ImageView>,

    command_pool: vk::CommandPool,

    depth_image: vk::Image,
    depth_image_view: vk::ImageView,
    depth_image_memory: vk::DeviceMemory,

    renderpass: vk::RenderPass,
    swapchain_framebuffers: Vec<vk::Framebuffer>,

    draw_command_buffers: [vk::CommandBuffer; 2],
    image_available_semaphores: [vk::Semaphore; 2],
    render_finished_semaphores: [vk::Semaphore; 2],
    in_flight_fences: [vk::Fence; 2],
    current_frame: usize,
    image_index: u32,

    memory_allocator: Allocator
}

impl Vust {
    pub const NAME: &'static str = "Vust";
    pub const C_NAME: &'static CStr = unsafe {
        CStr::from_bytes_with_nul_unchecked(b"Vust\0")
    };

    pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    /// used for vulkan
    // had to do all this funny shit cuz .parse() isnt const :/
    pub const VERSION_VK: u32 = {
        let version = env!("CARGO_PKG_VERSION").as_bytes();

        let major = version[0] as u32 - 48;
        let minor = version[2] as u32 - 48;
        let patch = version[4] as u32 - 48;

        vk::make_api_version(0, major, minor, patch)
    };

    pub const MAX_FRAMES_IN_FLIGHT: usize = 2;


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
                    .engine_version(Vust::VERSION_VK)
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

            let renderpass = device.create_render_pass(
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

            let swapchain_framebuffers = swapchain_image_views.iter().map(|image_view| {
                let attachments = [*image_view, depth_image_view];
                device.create_framebuffer(&vk::FramebufferCreateInfo::builder()
                    .render_pass(renderpass)
                    .attachments(&attachments)
                    .width(extent.width)
                    .height(extent.height)
                    .layers(1)
                    .build(), None).unwrap()
            }).collect::<Vec<_>>();
            #[cfg(debug_assertions)]
            println!("created swapchain framebuffers");

            let draw_command_buffers: [vk::CommandBuffer; 2] = device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_pool(command_pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(2)
                    .build()
            ).unwrap().try_into().unwrap();

            let semaphore_create_info = vk::SemaphoreCreateInfo::builder().build();
            let fence_create_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED).build();

            let image_available_semaphores = [device.create_semaphore(&semaphore_create_info, None).unwrap(), device.create_semaphore(&semaphore_create_info, None).unwrap()];
            let render_finished_semaphores = [device.create_semaphore(&semaphore_create_info, None).unwrap(), device.create_semaphore(&semaphore_create_info, None).unwrap()];
            let in_flight_fences = [device.create_fence(&fence_create_info, None).unwrap(), device.create_fence(&fence_create_info, None).unwrap()];

            let memory_allocator = Allocator::new(&AllocatorCreateDesc {
                instance: instance.clone(),
                device: device.clone(),
                physical_device,
                debug_settings: Default::default(),
                buffer_device_address: false,
                allocation_sizes: Default::default()
            }).unwrap();
            
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
                swapchain_util,
                swapchain,
                swapchain_image_views,
            
                command_pool,
            
                depth_image,
                depth_image_memory,
                depth_image_view,
            
                renderpass,
                swapchain_framebuffers,
            
                draw_command_buffers,
                image_available_semaphores,
                render_finished_semaphores,
                in_flight_fences,
                current_frame: 0,
                image_index: 0,
            
                memory_allocator
            }
        }
    }

    pub fn reset_command_buffer(&mut self) {
        unsafe {
            self.device.wait_for_fences(&[self.in_flight_fences[self.current_frame]], true, std::u64::MAX).unwrap();
            self.device.reset_fences(&[self.in_flight_fences[self.current_frame]]).unwrap();

            self.device.reset_command_buffer(self.draw_command_buffers[self.current_frame], vk::CommandBufferResetFlags::empty()).unwrap();

            self.image_index = self.swapchain_util.acquire_next_image(
                self.swapchain,
                std::u64::MAX,
                self.image_available_semaphores[self.current_frame],
                vk::Fence::null()
            ).unwrap().0;

            self.device.begin_command_buffer(self.draw_command_buffers[self.current_frame], &vk::CommandBufferBeginInfo::builder().build()).unwrap();

            self.device.cmd_begin_render_pass(
                self.draw_command_buffers[self.current_frame],
                &vk::RenderPassBeginInfo::builder()
                    .render_pass(self.renderpass)
                    .framebuffer(self.swapchain_framebuffers[self.image_index as usize])
                    .render_area(vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: self.extent
                    })
                    .clear_values(&[vk::ClearValue {
                        color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] }
                    }, vk::ClearValue {
                        depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 }
                    }])
                    .build(),
                vk::SubpassContents::INLINE
            );
        }
    }

    pub fn bind_pipeline(&self, pipeline_handle: vk::Pipeline) {
        unsafe {
            self.device.cmd_bind_pipeline(
                self.draw_command_buffers[self.current_frame],
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_handle
            );
        }
    }

    pub fn bind_viewport(&self, viewport: vk::Viewport) {
        unsafe {
            self.device.cmd_set_viewport(
                self.draw_command_buffers[self.current_frame],
                0,
                &[viewport]
            );
        }
    }

    pub fn bind_scissor(&self, scissor: vk::Rect2D) {
        unsafe {
            self.device.cmd_set_scissor(
                self.draw_command_buffers[self.current_frame],
                0,
                &[scissor]
            );
        }
    }

    pub fn bind_descriptor_set(&self, pipeline: &GraphicsPipeline) {
        unsafe {
            self.device.cmd_bind_descriptor_sets(
                self.draw_command_buffers[self.current_frame],
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.pipeline_layout(),
                0,
                &[pipeline.descriptor_sets().unwrap()[self.current_frame]],
                &[]
            );
        }
    }

    pub fn bind_vertex_buffer(&self, vertex_buffer: vk::Buffer) {
        unsafe {
            self.device.cmd_bind_vertex_buffers(
                self.draw_command_buffers[self.current_frame],
                0,
                &[vertex_buffer],
                &[0]
            );
        }
    }

    /// index buffer must contain 32bit integer (i32/u32) indices
    pub fn bind_index_buffer(&self, index_buffer: vk::Buffer) {
        unsafe {
            self.device.cmd_bind_index_buffer(
                self.draw_command_buffers[self.current_frame],
                index_buffer,
                0,
                vk::IndexType::UINT32
            );
        }
    }

    pub fn draw(&self, vertex_count: u32) {
        unsafe {
            self.device.cmd_draw(
                self.draw_command_buffers[self.current_frame],
                vertex_count,
                1,
                0,
                0
            );
        }
    }

    pub fn draw_indexed(&self, index_count: u32) {
        unsafe {
            self.device.cmd_draw_indexed(
                self.draw_command_buffers[self.current_frame],
                index_count,
                1,
                0,
                0,
                0
            );
        }
    }

    pub fn render_surface(&mut self) {
        unsafe {
            self.device.cmd_end_render_pass(self.draw_command_buffers[self.current_frame]);
            self.device.end_command_buffer(self.draw_command_buffers[self.current_frame]).unwrap();

            self.device.queue_submit(
                self.queue,
                &[
                    vk::SubmitInfo::builder()
                        .command_buffers(&[self.draw_command_buffers[self.current_frame]])
                        .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                        .wait_semaphores(&[self.image_available_semaphores[self.current_frame]])
                        .signal_semaphores(&[self.render_finished_semaphores[self.current_frame]])
                        .build()
                ],
                self.in_flight_fences[self.current_frame]
            ).unwrap();

            self.swapchain_util.queue_present(
                self.queue,
                &vk::PresentInfoKHR::builder()
                    .swapchains(&[self.swapchain])
                    .image_indices(&[self.image_index])
                    .wait_semaphores(&[self.render_finished_semaphores[self.current_frame]])
                    .build()
            ).unwrap();

            self.current_frame = (self.current_frame + 1) % Self::MAX_FRAMES_IN_FLIGHT;
        }
    }

    pub fn update_descriptor_set(&self, pipeline: &GraphicsPipeline, write_descriptor_infos: &[WriteDescriptorInfo]) {
        unsafe {
            let mut write_descriptor_info = pipeline.write_descriptor_set_infos()
                .iter()
                .map(|write_descriptor_infos| write_descriptor_infos[self.current_frame])
                .collect::<Vec<_>>();

            // im holding the infos in a vec for the duration of this function's scope to avoid ptr lifetime issues when i pass the pointer to buffer/image info into a vk::WriteDescriptorSet
            enum BufferOrImageInfo {
                Buffer(vk::DescriptorBufferInfo),
                Image(vk::DescriptorImageInfo)
            }

            let infos = write_descriptor_infos
                .iter()
                .map(|write_descriptor_info| {
                    match write_descriptor_info {
                        WriteDescriptorInfo::Buffer { buffer, offset, range } => {
                            BufferOrImageInfo::Buffer(vk::DescriptorBufferInfo::builder()
                                .buffer(*buffer)
                                .offset(*offset)
                                .range(*range)
                                .build())
                        }
                        WriteDescriptorInfo::Image { image_view, sampler } => {
                            BufferOrImageInfo::Image(vk::DescriptorImageInfo::builder()
                                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                                .image_view(*image_view)
                                .sampler(*sampler)
                                .build())
                        }
                    }
                })
                .collect::<Vec<BufferOrImageInfo>>();

            for (i, write_descriptor_info) in write_descriptor_info.iter_mut().enumerate() {
                match infos[i] {
                    BufferOrImageInfo::Buffer(buffer) => {
                        write_descriptor_info.p_buffer_info = &buffer;
                    }
                    BufferOrImageInfo::Image(image) => {
                        write_descriptor_info.p_image_info = &image;
                    }
                }
                write_descriptor_info.descriptor_count = 1;
            }

            self.device.update_descriptor_sets(
                &write_descriptor_info,
                &[]
            );
        }
    }

    pub fn wait_idle(&self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
        }
    }

    pub fn begin_single_exec_command(&self) -> vk::CommandBuffer {
        unsafe {
            let command_buffer = self.device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_pool(self.command_pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1)
                    .build(),
            ).unwrap()[0];
    
            self.device.begin_command_buffer(
                command_buffer,
                &vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
                    .build(),
            ).unwrap();
    
            command_buffer
        }
    }
    
    pub fn end_single_exec_command(&self, command_buffer: vk::CommandBuffer) {
        unsafe {
            self.device.end_command_buffer(command_buffer).unwrap();
    
            self.device.queue_submit(
                self.queue,
                &[
                    vk::SubmitInfo::builder()
                        .command_buffers(&[command_buffer])
                        .build(),
                ],
                vk::Fence::null()
            ).unwrap();
    
            self.device.queue_wait_idle(self.queue).unwrap();
    
            self.device.free_command_buffers(
                self.command_pool,
                &[command_buffer],
            );
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

    pub fn transition_image_layout(
        &self,
        image: vk::Image,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout
    ) {
        let transition_command_buffer = self.begin_single_exec_command();
    
        let mut barrier = vk::ImageMemoryBarrier::builder()
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1)
                .build())
            .build();
        
        let (src_stage, dst_stage) = if old_layout == vk::ImageLayout::UNDEFINED  && new_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL {
            barrier.src_access_mask = vk::AccessFlags::empty();
            barrier.dst_access_mask = vk::AccessFlags::TRANSFER_WRITE;
    
            (vk::PipelineStageFlags::TOP_OF_PIPE, vk::PipelineStageFlags::TRANSFER)
        } else if old_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL && new_layout == vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL {
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;
    
            (vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::FRAGMENT_SHADER)
        } else if old_layout == vk::ImageLayout::UNDEFINED && new_layout == vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL {
            barrier.subresource_range.aspect_mask = vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL;
            barrier.src_access_mask = vk::AccessFlags::empty();
            barrier.dst_access_mask = vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE;
    
            (vk::PipelineStageFlags::TOP_OF_PIPE, vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS)
        } else {
            unreachable!()
        };
    
        unsafe {
            self.device.cmd_pipeline_barrier(
                transition_command_buffer,
                src_stage,
                dst_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        }
    
        self.end_single_exec_command(transition_command_buffer);
    }
}
