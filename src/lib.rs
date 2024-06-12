pub mod create_info;

// expose the make_api_version function
pub use ash::vk::make_api_version;
use std::ffi::{CStr, CString};
use ash::vk;
use create_info::VustCreateInfo;

pub struct Vust {
    entry: ash::Entry,
    instance: ash::Instance
}

impl Vust {
    pub const NAME: &'static str = "Vust";
    pub const C_NAME: &'static CStr = unsafe {
        CStr::from_bytes_with_nul_unchecked(b"Vust\0")
    };
    pub const VERSION: u32 = vk::make_api_version(0, 0, 1, 0);


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
                    .engine_version(Vust::VERSION)
                    .api_version(vk::make_api_version(0, 1, 3, 0))
                    .build();

                #[cfg(debug_assertions)] {
                    // only enable debug utils in debug build
                    create_info.enabled_extensions.push(CString::new("VK_EXT_debug_utils").unwrap());
                    
                    println!("Enabled Instance Extensions: ");
                    for ext in &create_info.enabled_extensions {
                        println!("\t{}", ext.to_str().unwrap());
                    }
                }
                let enabled_extension_ptrs = create_info.enabled_extensions.iter().map(|ext| ext.as_ptr()).collect::<Vec<_>>();

                let enabled_layers = [CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
                let enabled_layer_ptrs = enabled_layers.iter().map(|layer| layer.as_ptr()).collect::<Vec<_>>();

                let instance_info = vk::InstanceCreateInfo::builder()
                    .application_info(&app_info)
                    .enabled_extension_names(&enabled_extension_ptrs)
                    .enabled_layer_names(&enabled_layer_ptrs)
                    .build();

                entry.create_instance(&instance_info, None).unwrap()
            };
            #[cfg(debug_assertions)]
            println!("Created Vulkan Instance");

            Self {
                entry,
                instance
            }
        }
    }
}
