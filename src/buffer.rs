use std::mem::size_of_val;

use ash::vk;
use gpu_allocator::{vulkan::{Allocation, AllocationCreateDesc, AllocationScheme}, MemoryLocation};

use crate::Vust;

pub struct Buffer {
    #[cfg(debug_assertions)]
    name: String,
    handle: vk::Buffer,
    memory: Allocation
}

impl Buffer {
    pub fn builder<'a, T>() -> BufferBuilder<'a, T> {
        BufferBuilder {
            #[cfg(debug_assertions)]
            name: "Default".to_string(),
            data: &[],
            usage: vk::BufferUsageFlags::empty(),
            memory_location: vk::MemoryPropertyFlags::empty()
        }
    }
    
    pub fn handle(&self) -> vk::Buffer {
        self.handle
    }
}

pub struct BufferBuilder<'a, T> {
    #[cfg(debug_assertions)]
    name: String,
    data: &'a [T],
    usage: vk::BufferUsageFlags,
    memory_location: vk::MemoryPropertyFlags
}

impl<'a, T> BufferBuilder<'a, T> {
    #[cfg(debug_assertions)]
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn with_data(mut self, data: &'a [T]) -> Self {
        self.data = data;
        self
    }

    pub fn with_usage(mut self, usage: vk::BufferUsageFlags) -> Self {
        self.usage = usage;
        self
    }

    pub fn with_memory_location(mut self, memory_location: vk::MemoryPropertyFlags) -> Self {
        self.memory_location = memory_location;
        self
    }

    /// write_on_creation - if true, the buffer will be written to on creation
    pub fn build(self, vust: &mut Vust, write_on_creation: bool) -> Buffer {
        unsafe {
            let buffer_create_info = vk::BufferCreateInfo::builder()
                .size(size_of_val(self.data) as u64)
                .usage(self.usage)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .build();

            let buffer = vust.device.create_buffer(&buffer_create_info, None).unwrap();

            let memory_requirements = vust.device.get_buffer_memory_requirements(buffer);
            
            let location = if self.memory_location.contains(vk::MemoryPropertyFlags::HOST_VISIBLE) {
                MemoryLocation::CpuToGpu
            } else if self.memory_location.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL) {
                MemoryLocation::GpuOnly
            } else {
                unimplemented!("not supported")
            };

            let memory_allocate_info = AllocationCreateDesc {
                #[cfg(debug_assertions)]
                name: &self.name,
                #[cfg(not(debug_assertions))]
                name: "not debug",
                requirements: memory_requirements,
                location,
                linear: false,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged
            };
            
            let memory = vust.memory_allocator.allocate(&memory_allocate_info).unwrap();

            vust.device.bind_buffer_memory(buffer, memory.memory(), memory.offset()).unwrap();

            #[cfg(debug_assertions)]
            println!("created buffer: {}", self.name);

            if write_on_creation {
                memory.mapped_ptr().unwrap().as_ptr().cast::<T>().copy_from_nonoverlapping(self.data.as_ptr(), self.data.len());
            }

            Buffer {
                #[cfg(debug_assertions)]
                name: self.name,
                handle: buffer,
                memory
            }
        }
    }
}