pub mod create_info;

// expose the make_api_version function
pub use ash::vk::make_api_version;
use std::ffi::{CStr, CString};
use ash::{extensions, vk};
use create_info::VustCreateInfo;

pub struct Vust {
    entry: ash::Entry,
    instance: ash::Instance,

    #[cfg(debug_assertions)]
    debug_utils_loader: extensions::ext::DebugUtils,
    #[cfg(debug_assertions)]
    debug_utils_messenger: vk::DebugUtilsMessengerEXT,

    physical_device: vk::PhysicalDevice
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
                    
                    println!("enabled instance extensions: ");
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
            println!("created vulkan instance");

            #[cfg(debug_assertions)]
            let (debug_utils_loader, debug_utils_messenger) = {
                let debug_utils_loader = extensions::ext::DebugUtils::new(&entry, &instance);
                let debug_utils_messenger = debug_utils_loader
                    .create_debug_utils_messenger(&vk::DebugUtilsMessengerCreateInfoEXT::builder()
                    .message_severity(
                        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR |
                        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING |
                        vk::DebugUtilsMessageSeverityFlagsEXT::INFO |
                        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    )
                    .message_type(
                        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL |
                        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE |
                        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    )
                    .pfn_user_callback(Some(Vust::vulkan_debug_callback))
                    .build(), None)
                    .unwrap();

                println!("created vulkan debug utils messenger");
                (debug_utils_loader, debug_utils_messenger)
            };

            let physical_devices = instance.enumerate_physical_devices().unwrap();
            let physical_device = physical_devices.into_iter().find(|physical_device| {
                let properties = instance.get_physical_device_properties(*physical_device);

                let physical_device_info = create_info::PhysicalDevice {
                    name: CStr::from_ptr(properties.device_name.as_ptr()).to_str().unwrap().to_string(),
                    device_type: match properties.device_type {
                        vk::PhysicalDeviceType::DISCRETE_GPU => create_info::PhysicalDeviceType::Discrete,
                        vk::PhysicalDeviceType::INTEGRATED_GPU => create_info::PhysicalDeviceType::Integrated,
                        _ => create_info::PhysicalDeviceType::NotSupported
                    }
                };

                (create_info.choose_physical_device)(physical_device_info)
            }).unwrap();

            #[cfg(debug_assertions)]
            println!("using physical device: {}", CStr::from_ptr(instance.get_physical_device_properties(physical_device).device_name.as_ptr()).to_str().unwrap());

            Self {
                entry,
                instance,

                #[cfg(debug_assertions)]
                debug_utils_loader,
                #[cfg(debug_assertions)]
                debug_utils_messenger,

                physical_device
            }
        }
    }

    /// yoinked from ash examples
    unsafe extern "system" fn vulkan_debug_callback(
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
        message_type: vk::DebugUtilsMessageTypeFlagsEXT,
        p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
        _user_data: *mut std::os::raw::c_void,
    ) -> vk::Bool32 {
        let callback_data = *p_callback_data;
        let message_id_number = callback_data.message_id_number;

        let message_id_name = if callback_data.p_message_id_name.is_null() {
            std::borrow::Cow::from("")
        } else {
            std::ffi::CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
        };

        let message = if callback_data.p_message.is_null() {
            std::borrow::Cow::from("")
        } else {
            std::ffi::CStr::from_ptr(callback_data.p_message).to_string_lossy()
        };

        println!(
            "{message_severity:?}:\n{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n",
        );

        vk::FALSE
    }
}
