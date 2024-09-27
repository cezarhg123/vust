pub use vk::{BufferUsageFlags, MemoryPropertyFlags};

use std::{mem::size_of_val, sync::{Arc, Mutex}};
use ash::vk;
use gpu_allocator::{vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator}, MemoryLocation};
use crate::Vust;

pub struct Buffer {
    #[cfg(debug_assertions)]
    name: String,
    handle: vk::Buffer,
    memory: Option<Allocation>,
    usage: vk::BufferUsageFlags,
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

    pub fn overwrite<T>(&self, data: &[T]) -> Result<(), &str> {
        if let Some(memory) = &self.memory {
            if memory.size() < size_of_val(data) as u64 {
                return Err("data size is bigger than buffer size");
            }

            unsafe {
                memory.mapped_ptr().unwrap().as_ptr().cast::<T>().copy_from_nonoverlapping(data.as_ptr(), data.len());
                Ok(())
            }
        } else {
            Err("buffer not created somehow??")
        }
    }

    pub fn destroy_raw(&mut self, device: ash::Device, memory_allocator: Arc<Mutex<Allocator>>) {
        unsafe {
            if !self.destroyed {
                device.destroy_buffer(self.handle, None);
                memory_allocator.lock().unwrap().free(self.memory.take().unwrap()).unwrap();
                self.destroyed = true;
            }
        }
    }

    pub fn destroy(&mut self, vust: &mut Vust) {
        self.destroy_raw(vust.device.clone(), Arc::clone(&vust.memory_allocator));
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

    #[cfg(not(debug_assertions))]
    pub fn with_name(mut self, _name: &str) -> Self {
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

    /// Build buffer using raw handles
    /// 
    /// currently only host visible and device local memory are supported
    /// 
    /// write_on_creation - if true, the buffer will be written to on creation
    pub fn build_raw(self, device: ash::Device, memory_allocator: Arc<Mutex<Allocator>>, write_on_creation: bool) -> Buffer {
        unsafe {
            let buffer_create_info = vk::BufferCreateInfo::builder()
                .size(size_of_val(self.data) as u64)
                .usage(self.usage)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .build();

            let buffer = device.create_buffer(&buffer_create_info, None).unwrap();

            let memory_requirements = device.get_buffer_memory_requirements(buffer);
            
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
            
            let memory = memory_allocator.lock().unwrap().allocate(&memory_allocate_info).unwrap();

            device.bind_buffer_memory(buffer, memory.memory(), memory.offset()).unwrap();

            if write_on_creation {
                memory.mapped_ptr().unwrap().as_ptr().cast::<T>().copy_from_nonoverlapping(self.data.as_ptr(), self.data.len());
            }

            Buffer {
                #[cfg(debug_assertions)]
                name: self.name,
                handle: buffer,
                memory: Some(memory),
                usage: self.usage,
                destroyed: false
            }
        }
    }

    /// Build buffer by passing Vust instance
    /// 
    /// currently only host visible and device local memory are supported
    /// 
    /// write_on_creation - if true, the buffer will be written to on creation
    pub fn build(self, vust: &mut Vust, write_on_creation: bool) -> Buffer {
        self.build_raw(vust.device.clone(), Arc::clone(&vust.memory_allocator), write_on_creation)
    }
}
