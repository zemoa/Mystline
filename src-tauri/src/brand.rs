pub fn tray_icon() -> tauri::image::Image<'static> {
    tauri::image::Image::new(include_bytes!("../icons/tray-32.rgba"), 32, 32)
}
