use ash::vk;

#[derive(Debug, Clone, Copy)]
pub enum WriteDescriptorInfo {
    Buffer {
        buffer: vk::Buffer,
        offset: vk::DeviceSize,
        range: vk::DeviceSize
    },
    Image {
        image_view: vk::ImageView,
        sampler: vk::Sampler
    }
}

impl WriteDescriptorInfo {
    pub fn to_vk(&self) -> (Option<vk::DescriptorBufferInfo>, Option<vk::DescriptorImageInfo>) {
        match self {
            WriteDescriptorInfo::Buffer { buffer, offset, range } => (
                Some(
                    vk::DescriptorBufferInfo::builder()
                        .buffer(*buffer)
                        .offset(*offset)
                        .range(*range)
                        .build()
                ),
                None
            ),
            WriteDescriptorInfo::Image { image_view, sampler } => (
                None,
                Some(
                    vk::DescriptorImageInfo::builder()
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .image_view(*image_view)
                        .sampler(*sampler)
                        .build()
                )
            )
        }
    }
}
