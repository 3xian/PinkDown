#[cfg(target_os = "windows")]
pub fn configure_native_window(window: &impl raw_window_handle::HasWindowHandle) -> bool {
    use raw_window_handle::RawWindowHandle;
    use windows_sys::Win32::{
        Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND},
        UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_STYLE, SWP_FRAMECHANGED,
            SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_CAPTION, WS_MAXIMIZEBOX,
            WS_MINIMIZEBOX, WS_SYSMENU, WS_THICKFRAME,
        },
    };

    let Ok(handle) = window.window_handle() else {
        return false;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return false;
    };
    let hwnd = handle.hwnd.get() as *mut core::ffi::c_void;

    // SAFETY: hwnd comes from eframe's live window handle. This function is only
    // called from App::update on the UI thread.
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        let style =
            (style & !WS_CAPTION) | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU;
        SetWindowLongPtrW(hwnd, GWL_STYLE, style as isize);

        let corner_preference = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            (&corner_preference as *const _) as *const core::ffi::c_void,
            std::mem::size_of_val(&corner_preference) as u32,
        );
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
    true
}
