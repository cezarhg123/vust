use std::ffi::CString;
use ash::vk::{self, VertexInputAttributeDescription, VertexInputBindingDescription};

pub struct VustCreateInfo {
    pub(super) app_name: CString,
    pub(super) app_version: u32,

    pub(super) enabled_instance_extensions: Vec<CString>,

    pub(super) choose_physical_device: fn(PhysicalDevice) -> bool,

    pub(super) surface_create_info: SurfaceCreateInfo,
    
    pub(super) framebuffer_size: (usize, usize),

    pub(super) graphics_pipeline_create_infos: Vec<GraphicsPipelineCreateInfo>
}

impl Default for VustCreateInfo {
    fn default() -> Self {
        Self {
            app_name: CString::new("Vust App").unwrap(),
            app_version: super::make_api_version(0, 1, 0, 0),
    
            enabled_instance_extensions: Vec::new(),

            choose_physical_device: |physical_device| {
                match physical_device.device_type {
                    PhysicalDeviceType::Discrete | PhysicalDeviceType::Integrated => true,
                    PhysicalDeviceType::NotSupported => false
                }
            },

            surface_create_info: SurfaceCreateInfo::None,

            framebuffer_size: (0, 0),

            graphics_pipeline_create_infos: Vec::new()
        }
    }
}

impl VustCreateInfo {
    pub fn with_app_name(mut self, app_name: &str) -> Self {
        self.app_name = CString::new(app_name).unwrap();
        self
    }

    pub fn with_app_version(mut self, app_version: u32) -> Self {
        self.app_version = app_version;
        self
    }

    pub fn with_extensions(mut self, extensions: Vec<impl Into<Vec<u8>>>) -> Self {
        self.enabled_instance_extensions = extensions.into_iter().map(|ext| CString::new(ext).unwrap()).collect();
        self
    }

    /// Optional - if not provided, will choose first discrete or integrated device
    pub fn with_choose_physical_device(mut self, choose_physical_device: fn(PhysicalDevice) -> bool) {
        self.choose_physical_device = choose_physical_device;
    }

    pub fn with_surface_create_info(mut self, surface_create_info: SurfaceCreateInfo) -> Self {
        self.surface_create_info = surface_create_info;
        self
    }

    pub fn with_framebuffer_size(mut self, framebuffer_size: (usize, usize)) -> Self {
        self.framebuffer_size = framebuffer_size;
        self
    }

    pub fn with_graphics_pipeline_create_infos(mut self, graphics_pipeline_create_infos: Vec<GraphicsPipelineCreateInfo>) -> Self {
        self.graphics_pipeline_create_infos = graphics_pipeline_create_infos;
        self
    }
}

pub struct PhysicalDevice {
    pub name: String,
    pub device_type: PhysicalDeviceType
}

pub enum PhysicalDeviceType {
    Discrete,
    Integrated,
    NotSupported
}

pub enum SurfaceCreateInfo {
    Win32 {
        hinstance: *const std::ffi::c_void,
        hwnd: *const std::ffi::c_void
    },
    None
}

impl SurfaceCreateInfo {
    pub fn into_win32(self) -> (*const std::ffi::c_void, *const std::ffi::c_void) {
        match self {
            SurfaceCreateInfo::Win32 { hinstance, hwnd } => (hinstance, hwnd),
            _ => panic!("surface create info is not win32")
        }
    }
}

pub struct GraphicsPipelineCreateInfo {
    pub name: String,
    pub vertex_bin: Vec<u8>,
    pub fragment_bin: Vec<u8>,
    pub enable_depth_test: bool,
    pub vertex_binding_descriptions: Vec<VertexInputBindingDescription>,
    pub vertex_attribute_descriptions: Vec<VertexInputAttributeDescription>,
    pub topology: vk::PrimitiveTopology,
    pub viewport: Viewport,
    pub scissor: Scissor,
    pub polygon_mode: vk::PolygonMode,
    pub cull_mode: CullMode,
    pub descriptor_set_layouts: Vec<DescriptorSetLayout>
}

#[derive(Debug, Clone)]
pub enum Viewport {
    Dynamic,
    Static {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        min_depth: f32,
        max_depth: f32
    }
}

#[derive(Debug, Clone)]
pub enum Scissor {
    Dynamic,
    Static {
        x: i32,
        y: i32,
        width: u32,
        height: u32
    }
}

pub enum CullMode {
    Clockwise,
    AntiClockwise,
    None
}

pub struct DescriptorSetLayout {
    /// Make sure to order the bindings correctly, as the index of the DescriptorSetBinding in the vector is used as the binding index for the descriptor binding.
    /// 
    /// e.g vec\!\[camera_binding, model_binding] = \[vk::DescriptorSetLayoutBinding { binding: 0, camera_binding info.. }, vk::DescriptorSetLayoutBinding { binding: 1, model_binding info.. }].
    pub bindings: Vec<DescriptorSetBinding>
}

pub struct DescriptorSetBinding {
    pub descriptor_type: vk::DescriptorType,
    pub stage_flags: vk::ShaderStageFlags
}
