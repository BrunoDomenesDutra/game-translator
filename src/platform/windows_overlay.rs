// game-translator/src/platform/windows_overlay.rs

// ============================================================================
// CONTROLE CLICK-THROUGH DA JANELA OVERLAY (WINDOWS)
// ============================================================================

#[cfg(windows)]
pub fn apply_click_through_mode(is_settings: bool) {
    if is_settings {
        remove_window_click_through();
    } else {
        // Reaplica click-through periodicamente (a cada ~500ms)
        use std::sync::atomic::{AtomicU64, Ordering};
        static LAST_CLICK_THROUGH: AtomicU64 = AtomicU64::new(0);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let last = LAST_CLICK_THROUGH.load(Ordering::Relaxed);
        if now - last > 500 {
            make_window_click_through();
            LAST_CLICK_THROUGH.store(now, Ordering::Relaxed);
        }
    }
}

#[cfg(not(windows))]
pub fn apply_click_through_mode(_is_settings: bool) {}

#[cfg(windows)]
fn make_window_click_through() {
    use winapi::um::winuser::{
        FindWindowW, GetWindowLongW, SetWindowLongW, GWL_EXSTYLE, WS_EX_LAYERED,
        WS_EX_TRANSPARENT,
    };

    unsafe {
        let title: Vec<u16> = "Ranmza Game Translator\0".encode_utf16().collect();
        let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());

        if !hwnd.is_null() {
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
            let new_style = ex_style | WS_EX_LAYERED as i32 | WS_EX_TRANSPARENT as i32;
            SetWindowLongW(hwnd, GWL_EXSTYLE, new_style);
            trace!("✅ Janela configurada como click-through!");
        } else {
            warn!("⚠️  Não foi possível encontrar a janela para click-through");
        }
    }
}

#[cfg(windows)]
fn remove_window_click_through() {
    use winapi::um::winuser::{
        FindWindowW, GetWindowLongW, SetWindowLongW, GWL_EXSTYLE, WS_EX_LAYERED,
        WS_EX_TRANSPARENT,
    };

    unsafe {
        let title: Vec<u16> = "Ranmza Game Translator\0".encode_utf16().collect();
        let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());

        if !hwnd.is_null() {
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
            let new_style = (ex_style | WS_EX_LAYERED as i32) & !(WS_EX_TRANSPARENT as i32);
            SetWindowLongW(hwnd, GWL_EXSTYLE, new_style);
        }
    }
}
