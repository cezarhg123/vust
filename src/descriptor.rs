use ash::vk;
use crate::Vust;

#[derive(Debug, Clone)]
pub struct Descriptor {
    pub(super) descriptor_pool: vk::DescriptorPool,
    pub(super) descriptor_set: [vk::DescriptorSet; Vust::MAX_FRAMES_IN_FLIGHT],
    pub(super) write_descriptor_set_info: Vec<[vk::WriteDescriptorSet; Vust::MAX_FRAMES_IN_FLIGHT]>
}
