use std::ptr::null;

use glfw::fail_on_errors;
use vust::{create_info::VustCreateInfo, Vust};
use winapi::um::libloaderapi::GetModuleHandleW;

#[test]
fn triangle() {
    let mut glfw = glfw::init(glfw::fail_on_errors!()).unwrap();
    glfw.window_hint(glfw::WindowHint::Resizable(false));
    glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));

    let (window, _) = glfw.create_window(800, 600, "Vust Triangle Test", glfw::WindowMode::Windowed).unwrap();

    let vust_create_info = VustCreateInfo::default()
        .with_app_name("Vust Triangle Test")
        .with_app_version(vust::make_api_version(0, 0, 1, 0))
        .with_extensions(glfw.get_required_instance_extensions().unwrap())
        .with_surface_create_info(
            vust::create_info::SurfaceCreateInfo::Win32 {
                hinstance: unsafe { GetModuleHandleW(null()).cast() },
                hwnd: window.get_win32_window()
            }
        );

    let vust = Vust::new(vust_create_info);

    while !window.should_close() {
        glfw.poll_events();
    }
}