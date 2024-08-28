pub use vk::{Format, Filter};

use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc};
use crate::{buffer::Buffer, Vust};

pub struct Texture {
    image: vk::Image,
    allocation: Option<Allocation>,
    view: vk::ImageView,
    sampler: vk::Sampler,
    descriptor_info: vk::DescriptorImageInfo,
    destroyed: bool
}

impl Texture {
    pub fn builder<'a>() -> TextureBuilder<'a> {
        TextureBuilder {
            #[cfg(debug_assertions)]
            name: "Default".to_string(),
            data: &[],
            dimensions: (0, 0),
            format: vk::Format::R8G8B8A8_SRGB,
            filter: vk::Filter::NEAREST
        }
    }

    pub fn destroy(&mut self, vust: &mut Vust) {
        if !self.destroyed {
            unsafe {
                vust.device.destroy_image(self.image, None);
                vust.memory_allocator.free(self.allocation.take().unwrap()).unwrap();
                vust.device.destroy_image_view(self.view, None);
                vust.device.destroy_sampler(self.sampler, None);
            }
        }
    }

    pub fn view(&self) -> vk::ImageView {
        self.view
    }

    pub fn sampler(&self) -> vk::Sampler {
        self.sampler
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        if !self.destroyed {
            panic!("texture was not destroyed");
        }
    }
}

pub struct TextureBuilder<'a> {
    #[cfg(debug_assertions)]
    name: String,
    data: &'a [u8],
    dimensions: (u32, u32),
    format: vk::Format,
    filter: vk::Filter
}

impl<'a> TextureBuilder<'a> {
    #[cfg(debug_assertions)]
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn with_data(mut self, data: &'a [u8]) -> Self {
        self.data = data;
        self
    }

    pub fn with_dimensions(mut self, dimensions: (u32, u32)) -> Self {
        self.dimensions = dimensions;
        self
    }

    pub fn with_format(mut self, format: vk::Format) -> Self {
        self.format = format;
        self
    }

    pub fn with_filter(mut self, filter: vk::Filter) -> Self {
        self.filter = filter;
        self
    }

    /// Returns None if data is empty
    pub fn build(self, vust: &mut Vust) -> Option<Texture> {
        if self.data.is_empty() {
            return None;
        } else {
            #[cfg(debug_assertions)]
            let data_buffer = Buffer::builder()
                .with_name(&self.name)
                .with_data(&self.data)
                .with_memory_location(vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT)
                .with_usage(vk::BufferUsageFlags::TRANSFER_SRC)
                .build(vust, true);

            #[cfg(not(debug_assertions))]
            let data_buffer = Buffer::builder()
                .with_data(&self.data)
                .with_memory_location(vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT)
                .with_usage(vk::BufferUsageFlags::TRANSFER_SRC)
                .build(vust, false);

            unsafe {
                let image = vust.device.create_image(
                    &vk::ImageCreateInfo::builder()
                        .image_type(vk::ImageType::TYPE_2D)
                        .extent(
                            vk::Extent3D {
                                width: self.dimensions.0,
                                height: self.dimensions.1,
                                depth: 1
                            }
                        )
                        .mip_levels(1)
                        .array_layers(1)
                        .format(self.format)
                        .tiling(vk::ImageTiling::OPTIMAL)
                        .initial_layout(vk::ImageLayout::UNDEFINED)
                        .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
                        .sharing_mode(vk::SharingMode::EXCLUSIVE)
                        .samples(vk::SampleCountFlags::TYPE_1)
                        .build(),
                    None
                ).unwrap();

                let requirements = vust.device.get_image_memory_requirements(image);

                #[cfg(debug_assertions)]
                let name = &self.name;
                #[cfg(not(debug_assertions))]
                let name = "texture";

                let allocation = vust.memory_allocator.allocate(
                    &AllocationCreateDesc {
                        name,
                        requirements,
                        location: gpu_allocator::MemoryLocation::GpuOnly,
                        linear: true,
                        allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged
                    }
                ).unwrap();

                vust.device.bind_image_memory(image, allocation.memory(), allocation.offset()).unwrap();

                vust.transition_image_layout(image, vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
                let copy_command_buffer = vust.begin_single_exec_command();
                vust.device.cmd_copy_buffer_to_image(
                    copy_command_buffer,
                    data_buffer.handle(),
                    image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[
                        vk::BufferImageCopy::builder()
                            .buffer_offset(0)
                            .buffer_row_length(0)
                            .buffer_image_height(0)
                            .image_subresource(
                                vk::ImageSubresourceLayers::builder()
                                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                                    .mip_level(0)
                                    .base_array_layer(0)
                                    .layer_count(1)
                                    .build()
                            )
                            .image_offset(
                                vk::Offset3D { x: 0, y: 0, z: 0 }
                            )
                            .image_extent(
                                vk::Extent3D { width: self.dimensions.0, height: self.dimensions.1, depth: 1 }
                            )
                            .build()
                    ]
                );
                vust.end_single_exec_command(copy_command_buffer);
                vust.transition_image_layout(image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

                let view = vust.device.create_image_view(
                    &vk::ImageViewCreateInfo::builder()
                        .image(image)
                        .view_type(vk::ImageViewType::TYPE_2D)
                        .format(self.format)
                        .subresource_range(
                            vk::ImageSubresourceRange::builder()
                                .aspect_mask(vk::ImageAspectFlags::COLOR)
                                .base_mip_level(0)
                                .level_count(1)
                                .base_array_layer(0)
                                .layer_count(1)
                                .build()
                        )
                        .build(),
                    None
                ).unwrap();

                let sampler = vust.device.create_sampler(
                    &vk::SamplerCreateInfo::builder()
                        .mag_filter(self.filter)
                        .min_filter(self.filter)
                        .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                        .address_mode_u(vk::SamplerAddressMode::REPEAT)
                        .address_mode_v(vk::SamplerAddressMode::REPEAT)
                        .address_mode_w(vk::SamplerAddressMode::REPEAT)
                        .anisotropy_enable(true)
                        .max_anisotropy(16.0)
                        .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
                        .unnormalized_coordinates(false)
                        .compare_enable(false)
                        .mip_lod_bias(0.0)
                        .min_lod(0.0)
                        .max_lod(0.0)
                        .build(),
                    None
                ).unwrap();

                let descriptor_info = vk::DescriptorImageInfo::builder()
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image_view(view)
                    .sampler(sampler)
                    .build();

                Some(Texture {
                    image,
                    allocation: Some(allocation),
                    view,
                    sampler,
                    descriptor_info,
                    destroyed: false
                })
            }
        }
    }
}
