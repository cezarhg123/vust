pub mod create_info;
pub mod buffer;
pub mod texture;
pub mod pipeline;
pub mod write_descriptor_info;
pub mod descriptor;
pub mod internal_vust;
pub mod vust_command;
pub mod vust_sync;

// expose a few ash/vk things
pub use ash::vk::{make_api_version, VertexInputBindingDescription, VertexInputAttributeDescription, Format, VertexInputRate, CommandPool};
pub use ash::Device;
pub use gpu_allocator::vulkan::Allocator;
pub use vk::{Viewport, Rect2D, Offset2D, Extent2D};
use create_info::VustCreateInfo;
use descriptor::Descriptor;
use gpu_allocator::vulkan::{Allocation, AllocatorCreateDesc};
use internal_vust::InternalVust;
use pipeline::GraphicsPipeline;
use vust_command::VustCommand;
use write_descriptor_info::WriteDescriptorInfo;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::{ffi::{CStr, CString}, sync::{Arc, Mutex}};
use ash::{extensions, vk};

use crate::vust_sync::VustSyncer;

/// This struct acts more like a handle and can be cloned and used anywhere
#[derive(Clone)]
pub struct Vust {
    device: ash::Device,
    memory_allocator: Arc<Mutex<Allocator>>,
    renderpass: vk::RenderPass,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    vust_sender: mpsc::Sender<VustCommand>
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

    pub fn new(create_info: VustCreateInfo) -> (Self, VustSyncer) {
        let mut vust = InternalVust::new(create_info);
        let device = vust.get_device();
        let memory_allocator = vust.get_memory_allocator();
        let renderpass = vust.get_renderpass();
        let command_pool = vust.get_command_pool();
        let queue = vust.get_queue();
        
        let (vust_sender, vust_receiver) = mpsc::channel();
        let (vust_sync_sender, vust_sync_receiver) = mpsc::channel::<()>();

        std::thread::spawn(move || {
            // take ownership
            let vust_sync_sender = vust_sync_sender;
            while let Ok(command) = vust_receiver.recv() {
                match command {
                    VustCommand::KYS => {
                        vust.wait_idle();
                        break;
                    }
                    command => vust.run(command, &vust_sync_sender)
                }
            }
        });

        (
            Self {
                device,
                memory_allocator,
                renderpass,
                command_pool,
                queue,
                vust_sender
            },
            VustSyncer {
                allow_messages_recv: vust_sync_receiver
            }
        )
    }

    pub fn destroy_buffer(&self, buffer: vk::Buffer, allocation: Allocation) {
        self.vust_sender.send(VustCommand::DestroyBuffer { buffer, allocation }).unwrap();
    }

    pub fn destroy_texture(&self, image: vk::Image, view: vk::ImageView, sampler: vk::Sampler, allocation: Allocation) {
        self.vust_sender.send(VustCommand::DestroyTexture { image, view, sampler, allocation }).unwrap();
    }

    pub fn reset_command_buffer(&self) {
        self.vust_sender.send(VustCommand::ResetCommandBuffer).unwrap();
    }

    pub fn bind_pipeline(&self, pipeline_handle: vk::Pipeline) {
        self.vust_sender.send(VustCommand::BindPipeline { pipeline_handle }).unwrap();
    }

    pub fn bind_viewport(&self, viewport: vk::Viewport) {
        self.vust_sender.send(VustCommand::BindViewport { viewport }).unwrap();
    }

    pub fn bind_scissor(&self, scissor: vk::Rect2D) {
        self.vust_sender.send(VustCommand::BindScissor { scissor }).unwrap();
    }

    pub fn bind_descriptor_set(&self, pipeline_layout: vk::PipelineLayout, descriptor: &Descriptor) {
        self.vust_sender.send(VustCommand::BindDescriptorSet { pipeline_layout, descriptor: descriptor.clone() }).unwrap();
    }

    pub fn bind_vertex_buffer(&self, vertex_buffer: vk::Buffer) {
        self.vust_sender.send(VustCommand::BindVertexBuffer { vertex_buffer }).unwrap();
    }

    pub fn bind_index_buffer(&self, index_buffer: vk::Buffer) {
        self.vust_sender.send(VustCommand::BindIndexBuffer { index_buffer }).unwrap();
    }

    pub fn draw(&self, vertex_count: u32) {
        self.vust_sender.send(VustCommand::Draw { vertex_count }).unwrap();
    }

    pub fn draw_indexed(&self, index_count: u32) {
        self.vust_sender.send(VustCommand::DrawIndexed { index_count }).unwrap();
    }

    pub fn update_descriptor_set(&self, descriptor: &Descriptor, write_descriptor_infos: Vec<WriteDescriptorInfo>) {
        self.vust_sender.send(VustCommand::UpdateDescriptorSet { descriptor: descriptor.clone(), write_descriptor_infos: write_descriptor_infos.clone() }).unwrap();
    }

    pub fn update_descriptor_set_once(&self, descriptor: &Descriptor, write_descriptor_infos: Vec<WriteDescriptorInfo>) {
        // 2 unsafe blocks to write both descriptors, theres a prettier/smarter way to do this but im lazy

        unsafe {
            let mut write_descriptor_info = descriptor.write_descriptor_set_info
                .iter()
                .map(|write_descriptor_infos| write_descriptor_infos[0])
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
                match &infos[i] {
                    BufferOrImageInfo::Buffer(buffer) => {
                        write_descriptor_info.p_buffer_info = buffer;
                    }
                    BufferOrImageInfo::Image(image) => {
                        write_descriptor_info.p_image_info = image;
                    }
                }
                write_descriptor_info.descriptor_count = 1;
            }

            self.device.update_descriptor_sets(
                &write_descriptor_info,
                &[]
            );
        }

        unsafe {
            let mut write_descriptor_info = descriptor.write_descriptor_set_info
                .iter()
                .map(|write_descriptor_infos| write_descriptor_infos[1])
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
                match &infos[i] {
                    BufferOrImageInfo::Buffer(buffer) => {
                        write_descriptor_info.p_buffer_info = buffer;
                    }
                    BufferOrImageInfo::Image(image) => {
                        write_descriptor_info.p_image_info = image;
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

    pub fn render_surface(&mut self) {
        self.vust_sender.send(VustCommand::RenderSurface).unwrap();
    }

    pub fn wait_idle(&self) {
        self.vust_sender.send(VustCommand::KYS).unwrap();
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
            let command_buffers = [command_buffer];
            self.device.queue_submit(
                self.queue,
                &[
                    vk::SubmitInfo::builder()
                        .command_buffers(&command_buffers)
                        .build(),
                ],
                vk::Fence::null()
            ).unwrap();
    
            self.device.queue_wait_idle(self.queue).unwrap();
    
            self.device.free_command_buffers(
                self.command_pool,
                &command_buffers,
            );
        }
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
        let barriers = [barrier];
        unsafe {
            self.device.cmd_pipeline_barrier(
                transition_command_buffer,
                src_stage,
                dst_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &barriers,
            );
        }
    
        self.end_single_exec_command(transition_command_buffer);
    }
}
