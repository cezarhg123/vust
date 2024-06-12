use std::ffi::CString;
use ash::vk;

pub struct VustCreateInfo {
    pub(super) app_name: CString,
    pub(super) app_version: u32,

    pub(super) enabled_extensions: Vec<CString>,

}

impl Default for VustCreateInfo {
    fn default() -> Self {
        Self {
            app_name: CString::new("Vust App").unwrap(),
            app_version: vk::make_api_version(0, 1, 0, 0),
    
            enabled_extensions: Vec::new()
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
        self.enabled_extensions = extensions.into_iter().map(|ext| CString::new(ext).unwrap()).collect();
        self
    }
}
