// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Force GDK_BACKEND=x11 for Wayland global hotkey support (uses XWayland)
    #[cfg(target_os = "linux")]
    {
        if std::env::var("GDK_BACKEND").is_err() {
            std::env::set_var("GDK_BACKEND", "x11");
            eprintln!("[WhisperShell] 🔧 Wayland detected — forcing GDK_BACKEND=x11 (XWayland) for global hotkey support");
        }
        
        // Fix for WebKitGTK transparent window disappearing on maximize/restore (Common Linux bug)
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    }

    // Check if any CLI command was invoked
    let args: Vec<String> = std::env::args().collect();
    let is_recording = args.iter().any(|arg| arg == "--toggle-recording");
    let is_config = args.iter().any(|arg| arg == "--toggle-config");
    let is_explicit_cli = is_recording || is_config;

    let socket_path = "/tmp/whispershell.sock";
    use std::io::Write;
    
    // Single-instance behavior: if we can connect to the socket, the app is already running in the background!
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(socket_path) {
        let signal: &[u8] = if is_recording { b"TOGGLE" } else { b"TOGGLE_CONFIG" };
        
        if let Err(e) = stream.write_all(signal) {
            eprintln!("[WhisperShell CLI] Failed to send signal to existing instance: {}", e);
            std::process::exit(1);
        }
        println!("[WhisperShell] An instance is already running. Triggered the existing instance.");
        std::process::exit(0);
    }

    if is_explicit_cli {
        // User explicitly tried to toggle via CLI, but the app isn't running
        eprintln!("[WhisperShell CLI] Failed to connect to daemon at {}. Is the app running?", socket_path);
        std::process::exit(1);
    }

    // App is not running, boot it normally
    whispershell_lib::run()
}
