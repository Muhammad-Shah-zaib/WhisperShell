// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Force GDK_BACKEND=x11 for Wayland global hotkey support (uses XWayland)
    #[cfg(target_os = "linux")]
    if std::env::var("GDK_BACKEND").is_err() {
        std::env::set_var("GDK_BACKEND", "x11");
        eprintln!("[WhisperShell] 🔧 Wayland detected — forcing GDK_BACKEND=x11 (XWayland) for global hotkey support");
    }

    whispershell_lib::run()
}
