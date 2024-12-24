use ash::vk;
use gpu_allocator::vulkan::Allocation;
use crate::{descriptor::Descriptor, write_descriptor_info::WriteDescriptorInfo};

pub enum VustCommand {
    KYS, // kill yourself

    DestroyBuffer {
        buffer: vk::Buffer,
        allocation: Allocation
    },
    DestroyTexture {
        image: vk::Image,
        view: vk::ImageView,
        sampler: vk::Sampler,
        allocation: Allocation
    },

    ResetCommandBuffer,
    BindPipeline {
        pipeline_handle: vk::Pipeline
    },
    BindViewport {
        viewport: vk::Viewport
    },
    BindScissor {
        scissor: vk::Rect2D
    },
    BindDescriptorSet {
        pipeline_layout: vk::PipelineLayout,
        descriptor: Descriptor // probably should be arc or something but cloning is fine for now
    },
    BindVertexBuffer {
        vertex_buffer: vk::Buffer
    },
    BindIndexBuffer {
        index_buffer: vk::Buffer
    },
    Draw {
        vertex_count: u32
    },
    DrawIndexed {
        index_count: u32
    },
    UpdateDescriptorSet {
        descriptor: Descriptor,
        write_descriptor_infos: Vec<WriteDescriptorInfo>
    },
    RenderSurface
}

unsafe impl Send for VustCommand {}

pub(crate) struct DestroyBuffer {
    pub(crate) buffer: vk::Buffer,
    pub(crate) allocation: Allocation
}

pub(crate) struct DestroyTexture {
    pub(crate) image: vk::Image,
    pub(crate) view: vk::ImageView,
    pub(crate) sampler: vk::Sampler,
    pub(crate) allocation: Allocation
}