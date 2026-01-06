use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use winapi::ctypes::c_void;
use winapi::shared::minwindef::{BOOL, LPARAM, TRUE};
use winapi::shared::windef::HWND;
use winapi::um::dwmapi::DwmSetWindowAttribute;
use winapi::um::winuser::{
    EnumWindows, GetWindowTextA, SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    SetWindowPos,
};

const APP_TITLE: &str = "DMA Speed Test";

pub fn enable_dark_mode_for_all() -> Option<&'static str> {
    let found = Arc::new(AtomicBool::new(false));
    let found_clone = found.clone();

    // SAFETY: We are using WinAPI to enumerate windows. The callback is safe to define here.
    // The pointer passed to EnumWindows is valid and kept alive by the Arc.
    unsafe {
        extern "system" fn enum_callback(hwnd: HWND, found: LPARAM) -> BOOL {
            // SAFETY: hwnd is guaranteed to be a valid window handle by EnumWindows.
            let window_title = unsafe { get_window_title(hwnd) };

            if let Some(window_title) = window_title
                && window_title.contains(APP_TITLE)
            {
                // SAFETY: hwnd is valid.
                unsafe { enable_dark_mode(hwnd) };
                // SAFETY: found is the LPARAM passed to EnumWindows, which is a pointer to our AtomicBool.
                unsafe { mark_window_found(found) };
            }
            TRUE
        }

        EnumWindows(Some(enum_callback), &*found_clone as *const _ as LPARAM);
    }

    if found.load(Ordering::SeqCst) {
        Some("Dark mode set successfully")
    } else {
        None
    }
}

unsafe fn get_window_title(hwnd: HWND) -> Option<String> {
    // SAFETY: We provide a valid buffer and size to GetWindowTextA.
    unsafe {
        let mut title = vec![0u8; 512];
        let len = GetWindowTextA(hwnd, title.as_mut_ptr() as *mut i8, title.len() as i32);
        if len == 0 {
            return None;
        }
        title.truncate(len as usize);
        std::ffi::CString::new(title)
            .ok()
            .map(|c_string| c_string.to_string_lossy().into_owned())
    }
}

unsafe fn enable_dark_mode(hwnd: HWND) {
    // SAFETY: We are calling WinAPI functions with a valid HWND provided by the caller.
    unsafe {
        let dark_mode = TRUE;
        DwmSetWindowAttribute(
            hwnd,
            20, // DWMWA_USE_IMMERSIVE_DARK_MODE
            &dark_mode as *const _ as *const c_void,
            std::mem::size_of::<BOOL>() as u32,
        );

        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
        );
    }
}

unsafe fn mark_window_found(found: LPARAM) {
    // SAFETY: The caller must ensure `found` is a valid pointer to an AtomicBool.
    unsafe {
        let found_ptr = found as *mut AtomicBool;
        (*found_ptr).store(true, Ordering::SeqCst);
    }
}

pub fn setup_window_controls() {
    std::thread::spawn(|| {
        if enable_dark_mode_for_all().is_some() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
        for _ in 0..20 {
            if enable_dark_mode_for_all().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });
}
