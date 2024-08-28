pub use vk::{BufferUsageFlags, MemoryPropertyFlags};

use std::mem::size_of_val;
use ash::vk::{self, Handle};
use gpu_allocator::{vulkan::{Allocation, AllocationCreateDesc, AllocationScheme}, MemoryLocation};
use crate::Vust;

pub struct Buffer {
    #[cfg(debug_assertions)]
    name: String,
    handle: vk::Buffer,
    memory: Option<Allocation>,
    usage: vk::BufferUsageFlags,
    /// different from size of memory (self.memory.size())
    buffer_size: u64,
    destroyed: bool
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

    /// Allocates new memory if size of data is more than current buffer size
    /// 
    /// If data size isnt the same as current buffer size, you might aswell just create a new buffer
    pub fn change_data<T>(&mut self, vust: &mut Vust, data: &[T]) {
        unsafe {
            let memory = self.memory.as_mut().unwrap();

            let data_size = size_of_val(data) as u64;
            if memory.size() >= data_size {
                if self.buffer_size > data_size {
                    vust.device.destroy_buffer(self.handle, None);
                    
                    let buffer = vust.device.create_buffer(
                        &vk::BufferCreateInfo::builder()
                            .size(data_size)
                            .usage(self.usage)
                            .sharing_mode(vk::SharingMode::EXCLUSIVE)
                            .build(),
                        None
                    ).unwrap();

                    self.handle = buffer;
                    self.buffer_size = data_size;
                    vust.device.bind_buffer_memory(buffer, memory.memory(), memory.offset()).unwrap();
                }

                memory.mapped_ptr().unwrap().as_ptr().cast::<T>().copy_from_nonoverlapping(data.as_ptr(), data.len());
            } else {
                vust.device.destroy_buffer(self.handle, None);

                let buffer = vust.device.create_buffer(
                    &vk::BufferCreateInfo::builder()
                        .size(data_size)
                        .usage(self.usage)
                        .sharing_mode(vk::SharingMode::EXCLUSIVE)
                        .build(),
                    None
                ).unwrap();

                self.handle = buffer;
                self.buffer_size = data_size;

                *memory = vust.memory_allocator.allocate(
                    &AllocationCreateDesc {
                        #[cfg(debug_assertions)]
                        name: &self.name,
                        #[cfg(not(debug_assertions))]
                        name: "not debug",
                        requirements: vust.device.get_buffer_memory_requirements(self.handle),
                        location: MemoryLocation::CpuToGpu,
                        linear: false,
                        allocation_scheme: AllocationScheme::GpuAllocatorManaged
                    }
                ).unwrap();

                vust.device.bind_buffer_memory(self.handle, memory.memory(), memory.offset()).unwrap();

                memory.mapped_ptr().unwrap().as_ptr().cast::<T>().copy_from_nonoverlapping(data.as_ptr(), data.len());
            }
        }
    }

    pub fn destroy(&mut self, vust: &mut Vust) {
        unsafe {
            if !self.destroyed {
                vust.device.destroy_buffer(self.handle, None);
                vust.memory_allocator.free(self.memory.take().unwrap()).unwrap();
                self.destroyed = true;
            }
        }
    }

    pub fn handle(&self) -> vk::Buffer {
        self.handle
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if !self.destroyed {
            panic!("buffer was not destroyed");
        }
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

    /// currently only host visible and device local memory are supported
    /// 
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
            println!("created buffer: {} - handle: {:#x}", self.name, buffer.as_raw());

            if write_on_creation {
                memory.mapped_ptr().unwrap().as_ptr().cast::<T>().copy_from_nonoverlapping(self.data.as_ptr(), self.data.len());
            }

            Buffer {
                #[cfg(debug_assertions)]
                name: self.name,
                handle: buffer,
                memory: Some(memory),
                usage: self.usage,
                buffer_size: memory_requirements.size,
                destroyed: false
            }
        }
    }
}
