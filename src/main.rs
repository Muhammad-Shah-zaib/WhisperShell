// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Force GDK_BACKEND=x11 for Wayland global hotkey support (uses XWayland)
    #[cfg(target_os = "linux")]
    if std::env::var("GDK_BACKEND").is_err() {
        std::env::set_var("GDK_BACKEND", "x11");
        eprintln!("[WhisperShell] 🔧 Wayland detected — forcing GDK_BACKEND=x11 (XWayland) for global hotkey support");
    }

    // Check if any CLI command was invoked
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--toggle-recording" || arg == "--toggle-config") {
        let is_recording = args.iter().any(|arg| arg == "--toggle-recording");
        let signal: &[u8] = if is_recording { b"TOGGLE" } else { b"TOGGLE_CONFIG" };
        
        let socket_path = "/tmp/whispershell.sock";
        use std::io::Write;
        match std::os::unix::net::UnixStream::connect(socket_path) {
            Ok(mut stream) => {
                if let Err(e) = stream.write_all(signal) {
                    eprintln!("[WhisperShell CLI] Failed to send signal: {}", e);
                    std::process::exit(1);
                }
                println!("[WhisperShell CLI] Signal sent successfully.");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("[WhisperShell CLI] Failed to connect to daemon at {}. Is the app running? Error: {}", socket_path, e);
                std::process::exit(1);
            }
        }
    }

    whispershell_lib::run()
}
