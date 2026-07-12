mod audio;
mod constants;
mod inference;
mod injection;

use audio::AudioRecorder;
use inference::{load_whisper_context, transcribe};
use injection::copy_to_clipboard;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};

use whisper_rs::WhisperContext;

// ── App state ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum RecordingState {
    Idle,
    Recording,
    Processing,
}

pub struct AppState {
    pub recording_state: RecordingState,
    pub recorder: Option<AudioRecorder>,
    pub whisper_ctx: Option<WhisperContext>,
    pub last_transcript: String,
    pub download_cancel_flag: Option<Arc<AtomicBool>>,
}

impl AppState {
    fn new() -> Self {
        AppState {
            recording_state: RecordingState::Idle,
            recorder: None,
            whisper_ctx: None,
            last_transcript: String::new(),
            download_cancel_flag: None,
        }
    }
}

// Tauri requires state types to be Send + Sync
unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

// ── Tauri commands ─────────────────────────────────────────────────────────

use constants::HOTKEY_DISPLAY;

#[tauri::command]
fn get_app_info(app: tauri::AppHandle) -> serde_json::Value {
    let config = load_config(app);
    let model_key = config.get("model").and_then(|v| v.as_str()).unwrap_or("parakeet");
    
    serde_json::json!({
        "model": get_model_display_name(model_key),
        "hotkey": HOTKEY_DISPLAY,
    })
}

#[tauri::command]
fn log_to_terminal(msg: String) {
    println!("[Frontend] {}", msg);
}

#[tauri::command]
fn get_status(state: tauri::State<Arc<Mutex<AppState>>>) -> serde_json::Value {
    let s = state.lock().unwrap();
    serde_json::json!({
        "state": format!("{:?}", s.recording_state),
        "transcript": s.last_transcript,
    })
}

#[tauri::command]
fn select_directory() -> Option<String> {
    rfd::FileDialog::new()
        .pick_folder()
        .map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
fn select_file() -> Option<String> {
    rfd::FileDialog::new()
        .pick_file()
        .map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
fn get_installed_models(app: tauri::AppHandle) -> Vec<String> {
    let mut models = Vec::new();
    if let Ok(data_dir) = app.path().app_data_dir() {
        let models_dir = data_dir.join("models");
        if let Ok(entries) = std::fs::read_dir(models_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.ends_with(".bin") {
                                models.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    models
}

#[derive(serde::Serialize, Clone)]
struct DownloadProgress {
    model_id: String,
    progress: f64,
    downloaded: u64,
    total: u64,
}

#[tauri::command]
async fn download_model(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    model_id: String,
    filename: String,
) -> Result<(), String> {
    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut s = state.lock().unwrap();
        s.download_cancel_flag = Some(cancel_flag.clone());
    }

    let url = if filename.starts_with("ggml-") {
        format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}", filename)
    } else {
        // Fallback for non-ggerganov models if hosted elsewhere
        format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}", filename) 
    };

    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let models_dir = data_dir.join("models");
    std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;

    let tmp_path = models_dir.join(format!("{}.tmp", filename));
    let final_path = models_dir.join(&filename);

    let mut response = reqwest::get(&url).await.map_err(|e| e.to_string())?;
    let total_size = response.content_length().unwrap_or(0);

    let mut file = std::fs::File::create(&tmp_path).map_err(|e| e.to_string())?;
    use std::io::Write;
    use futures_util::StreamExt;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(item) = stream.next().await {
        if cancel_flag.load(Ordering::SeqCst) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err("Download cancelled".to_string());
        }
        let chunk = item.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        let progress = if total_size > 0 {
            (downloaded as f64 / total_size as f64) * 100.0
        } else {
            0.0
        };

        let _ = app.emit("download_progress", DownloadProgress {
            model_id: model_id.clone(),
            progress,
            downloaded,
            total: total_size,
        });
    }

    if cancel_flag.load(Ordering::SeqCst) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err("Download cancelled".to_string());
    }

    std::fs::rename(&tmp_path, &final_path).map_err(|e| e.to_string())?;

    let _ = app.emit("download_complete", model_id);

    {
        let mut s = state.lock().unwrap();
        s.download_cancel_flag = None;
    }

    Ok(())
}

#[tauri::command]
fn cancel_download(state: tauri::State<Arc<Mutex<AppState>>>) -> Result<(), String> {
    let s = state.lock().unwrap();
    if let Some(flag) = &s.download_cancel_flag {
        flag.store(true, Ordering::SeqCst);
    }
    Ok(())
}


#[tauri::command]
fn load_config(app: tauri::AppHandle) -> serde_json::Value {
    if let Ok(config_dir) = app.path().app_config_dir() {
        let config_path = config_dir.join("config.json");
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(json) = serde_json::from_str(&content) {
                return json;
            }
        }
    }
    
    // Default config
    serde_json::json!({
        "model": "parakeet",
        "history_limit": "5",
        "overlay_size": "small",
        "voice_recordings_dir": "~/.local/share/whispershell/recordings",
        "messages_log_file": "~/.local/share/whispershell/messages.log",
        "error_logs_dir": "~/.local/state/whispershell/errors"
    })
}

#[tauri::command]
fn save_config(app: tauri::AppHandle, state: tauri::State<Arc<Mutex<AppState>>>, config: serde_json::Value) -> Result<(), String> {
    let old_config = load_config(app.clone());
    let old_model = old_config.get("model").and_then(|v| v.as_str()).unwrap_or("parakeet").to_string();
    let new_model = config.get("model").and_then(|v| v.as_str()).unwrap_or("parakeet").to_string();
    
    let model_changed = old_model != new_model;

    // --- Migration Logic ---
    let old_voice_dir = expand_tilde(old_config.get("voice_recordings_dir").and_then(|v| v.as_str()).unwrap_or(""));
    let new_voice_dir = expand_tilde(config.get("voice_recordings_dir").and_then(|v| v.as_str()).unwrap_or(""));
    if old_voice_dir != new_voice_dir && old_voice_dir.exists() && !new_voice_dir.as_os_str().is_empty() {
        let _ = std::fs::create_dir_all(&new_voice_dir);
        if let Ok(entries) = std::fs::read_dir(&old_voice_dir) {
            for entry in entries.flatten() {
                if let Ok(ft) = entry.file_type() {
                    if ft.is_file() && entry.path().extension().and_then(|s| s.to_str()) == Some("wav") {
                        let new_path = new_voice_dir.join(entry.file_name());
                        if std::fs::copy(entry.path(), &new_path).is_ok() {
                            let _ = std::fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
    }

    let old_msg_log = expand_tilde(old_config.get("messages_log_file").and_then(|v| v.as_str()).unwrap_or(""));
    let new_msg_log = expand_tilde(config.get("messages_log_file").and_then(|v| v.as_str()).unwrap_or(""));
    if old_msg_log != new_msg_log && old_msg_log.exists() && !new_msg_log.as_os_str().is_empty() {
        if let Some(parent) = new_msg_log.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if std::fs::copy(&old_msg_log, &new_msg_log).is_ok() {
            let _ = std::fs::remove_file(&old_msg_log);
        }
    }

    let old_err_dir = expand_tilde(old_config.get("error_logs_dir").and_then(|v| v.as_str()).unwrap_or(""));
    let new_err_dir = expand_tilde(config.get("error_logs_dir").and_then(|v| v.as_str()).unwrap_or(""));
    if old_err_dir != new_err_dir && old_err_dir.exists() && !new_err_dir.as_os_str().is_empty() {
        let _ = std::fs::create_dir_all(&new_err_dir);
        if let Ok(entries) = std::fs::read_dir(&old_err_dir) {
            for entry in entries.flatten() {
                if let Ok(ft) = entry.file_type() {
                    if ft.is_file() {
                        let new_path = new_err_dir.join(entry.file_name());
                        if std::fs::copy(entry.path(), &new_path).is_ok() {
                            let _ = std::fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
    }
    // --- End Migration Logic ---

    // --- Prune according to new history limit ---
    let new_history_limit: usize = config.get("history_limit").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(5);
    
    if new_voice_dir.exists() && !new_voice_dir.as_os_str().is_empty() {
        prune_directory(&new_voice_dir, new_history_limit);
    }
    if new_msg_log.exists() && !new_msg_log.as_os_str().is_empty() {
        prune_log_file(&new_msg_log, new_history_limit);
    }
    if new_err_dir.exists() && !new_err_dir.as_os_str().is_empty() {
        let err_log = new_err_dir.join("errors.log");
        if err_log.exists() {
            prune_log_file(&err_log, new_history_limit);
        }
    }

    let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    let config_path = config_dir.join("config.json");
    let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(config_path, content).map_err(|e| e.to_string())?;

    if model_changed {
        let app_handle = app.clone();
        let state_for_loader = state.inner().clone();
        
        std::thread::spawn(move || {
            let _ = app_handle.emit("model_loading", ());
            let model_path = resolve_model_path(&app_handle);
            println!("[WhisperShell] Dynamic swap: Loading model from: {}", model_path.display());
            
            match load_whisper_context(&model_path) {
                Ok(ctx) => {
                    {
                        let mut s = state_for_loader.lock().unwrap();
                        s.whisper_ctx = Some(ctx);
                    }
                    println!("[WhisperShell] ✅ Dynamic model swap complete!");
                    let _ = app_handle.emit("model_ready", get_model_display_name(&new_model));
                }
                Err(e) => {
                    eprintln!("[WhisperShell] ❌ Dynamic model swap failed: {e}");
                    let _ = app_handle.emit("whisper_error", format!("Model load failed: {e}"));
                }
            }
        });
    }

    let _ = app.emit("config_updated", ());

    Ok(())
}


// ── Helpers ────────────────────────────────────────────────────────────────

fn resolve_model_path(app: &tauri::AppHandle) -> PathBuf {
    let config = load_config(app.clone());
    let selected_model = config.get("model").and_then(|v| v.as_str()).unwrap_or("parakeet");
    
    let filename = match selected_model {
        "base" => "ggml-base.en.bin",
        "turbo" => "ggml-large-v3-turbo.bin",
        "large" => "ggml-large-v3.bin",
        "parakeet" => "parakeet-v3.bin",
        _ => "parakeet-v3.bin",
    };

    app.path().app_data_dir().unwrap_or_default().join("models").join(filename)
}

fn get_model_display_name(model_key: &str) -> &'static str {
    match model_key {
        "base" => "Whisper Base",
        "turbo" => "Whisper Turbo",
        "large" => "Whisper Large v3",
        "parakeet" => "Parakeet v3",
        _ => "Unknown Model",
    }
}

// ── Overlay helpers ────────────────────────────────────────────────────────

fn show_overlay(app: &AppHandle) {
    if let Some(overlay) = app.get_webview_window("overlay") {
        // Re-assert always-on-top on every show
        let _ = overlay.set_always_on_top(true);

        // Since XWayland cursor position is stale on GNOME Wayland (it only updates when hovering over X11 windows),
        // we use a Wayland trick to find the true active monitor: we spawn a tiny, invisible dummy window.
        // GNOME's window manager natively places new windows on the active monitor (where the cursor or focus is).
        // We then read its monitor, close it immediately, and use that as our target!
        let mut target_monitor = None;
        if let Ok(dummy) = tauri::WebviewWindowBuilder::new(
            app,
            "dummy_probe",
            tauri::WebviewUrl::App("overlay.html".into())
        )
        .inner_size(1.0, 1.0)
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(true)
        .build() {
            target_monitor = dummy.current_monitor().ok().flatten();
            let _ = dummy.close();
        }

        // Fallback to current monitor or primary if cursor logic fails
        let monitor_result = target_monitor.or_else(|| {
            overlay
                .current_monitor()
                .ok()
                .flatten()
                .or_else(|| app.primary_monitor().ok().flatten())
        });
        
        // Show the overlay FIRST so the window manager maps it
        let _ = overlay.show();

        if let Some(monitor) = monitor_result {
            let scale   = monitor.scale_factor();
            let mon_pos = monitor.position();
            let mon_sz  = monitor.size();

            // Window logical size (from tauri.conf.json): 320 × 80
            let win_w  = (320.0 * scale) as u32;
            let win_h  = (80.0  * scale) as u32;
            
            // To mimic CSS "bottom: 15%", we calculate the margin dynamically based on the monitor's height.
            // This ensures it looks proportionally lifted on ANY screen resolution (4K, 1080p, etc.)
            let margin = (mon_sz.height as f64 * 0.15) as u32; 

            // This math does exactly what CSS `left: 50%; transform: translateX(-50%)` does!
            let x = mon_pos.x + ((mon_sz.width.saturating_sub(win_w)) / 2) as i32;
            let y = mon_pos.y + (mon_sz.height.saturating_sub(win_h + margin)) as i32;

            // Move the window AFTER it's mapped to override GNOME's auto-placement
            let _ = overlay.set_position(PhysicalPosition::new(x, y));
            
            // In XWayland/GNOME, sometimes setting focus helps or setting position again
            let _ = overlay.set_focus();
        }
    }
}

fn hide_overlay(app: &AppHandle) {
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.hide();
    }
}

// ── Hotkey toggle logic ────────────────────────────────────────────────────

fn handle_hotkey_press(app: &AppHandle, shared_state: Arc<Mutex<AppState>>) {
    let current_state = {
        let s = shared_state.lock().unwrap();
        s.recording_state.clone()
    };

    println!("[WhisperShell] 🔔 handle_hotkey_press — state: {:?}", current_state);

    match current_state {
        RecordingState::Idle => {
            println!("[WhisperShell] ▶️  Starting recording...");
            let mut s = shared_state.lock().unwrap();
            let mut recorder = match AudioRecorder::new() {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("[WhisperShell] Audio init error: {e}");
                    let _ = app.emit("whisper_error", e);
                    return;
                }
            };
            if let Err(e) = recorder.start(app.clone()) {
                eprintln!("[WhisperShell] Start recording error: {e}");
                let _ = app.emit("whisper_error", e);
                return;
            }
            s.recorder = Some(recorder);
            s.recording_state = RecordingState::Recording;
            let _ = app.emit("state_changed", "Recording");
            show_overlay(app);
        }

        RecordingState::Recording => {
            println!("[WhisperShell] ⏹️  Stopping recording — starting transcription...");
            let audio = {
                let mut s = shared_state.lock().unwrap();
                s.recording_state = RecordingState::Processing;
                let _ = app.emit("state_changed", "Processing");
                match s.recorder.as_mut() {
                    Some(rec) => rec.stop_and_get_audio(),
                    None      => Err("No active recorder".into()),
                }
            };

            let audio = match audio {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("[WhisperShell] Audio stop error: {e}");
                    let _ = app.emit("whisper_error", e.clone());
                    shared_state.lock().unwrap().recording_state = RecordingState::Idle;
                    let _ = app.emit("state_changed", "Idle");
                    return;
                }
            };

            // Transcribe — CPU/GPU intensive; already on a background thread
            let transcript = {
                let s = shared_state.lock().unwrap();
                match &s.whisper_ctx {
                    Some(ctx) => transcribe(ctx, &audio),
                    None      => Err("Whisper model not loaded".into()),
                }
            };

            let config = load_config(app.clone());
            let history_limit: usize = config.get("history_limit").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(5);
            let voice_dir = expand_tilde(config.get("voice_recordings_dir").and_then(|v| v.as_str()).unwrap_or(""));
            let msg_log = expand_tilde(config.get("messages_log_file").and_then(|v| v.as_str()).unwrap_or(""));
            let err_dir = expand_tilde(config.get("error_logs_dir").and_then(|v| v.as_str()).unwrap_or(""));

            let mut s = shared_state.lock().unwrap();
            match transcript {
                Ok(text) => {
                    println!("[WhisperShell] Transcript: {text}");
                    if let Err(e) = copy_to_clipboard(&text) {
                        eprintln!("[WhisperShell] Clipboard error: {e}");
                    }
                    s.last_transcript = text.clone();
                    let _ = app.emit("transcript_ready", text.clone());

                    if !voice_dir.as_os_str().is_empty() {
                        let _ = std::fs::create_dir_all(&voice_dir);
                        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                        let audio_path = voice_dir.join(format!("recording_{}.wav", timestamp));
                        if let Err(e) = crate::audio::save_wav(&audio, &audio_path) {
                            eprintln!("[WhisperShell] Failed to save wav: {}", e);
                        }
                        prune_directory(&voice_dir, history_limit);
                    }

                    if !msg_log.as_os_str().is_empty() {
                        if let Some(parent) = msg_log.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        use std::io::Write;
                        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&msg_log) {
                            let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                            let _ = writeln!(f, "[{}] {}", timestamp, text);
                        }
                        prune_log_file(&msg_log, history_limit);
                    }

                    // Auto-dismiss overlay after 2 s
                    let app_clone = app.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_secs(2));
                        hide_overlay(&app_clone);
                    });
                }
                Err(e) => {
                    eprintln!("[WhisperShell] Transcription error: {e}");
                    let _ = app.emit("whisper_error", e.clone());
                    hide_overlay(app);
                    
                    if !err_dir.as_os_str().is_empty() {
                        let _ = std::fs::create_dir_all(&err_dir);
                        use std::io::Write;
                        let err_log = err_dir.join("errors.log");
                        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&err_log) {
                            let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                            let _ = writeln!(f, "[{}] {}", timestamp, e);
                        }
                        prune_log_file(&err_log, history_limit);
                    }
                }
            }

            s.recording_state = RecordingState::Idle;
            let _ = app.emit("state_changed", "Idle");
        }

        RecordingState::Processing => {
            println!("[WhisperShell] Still processing — ignoring hotkey");
        }
    }
}

// ── Tauri entry point ──────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shared_state = Arc::new(Mutex::new(AppState::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"])
        ))
        .manage(shared_state.clone())
        .setup(move |app| {
            use tauri_plugin_autostart::ManagerExt;
            
            // Auto-enable autostart if it's not explicitly disabled by user
            // In a real app we'd track this in config, but since user requested "open by default", we enforce it here:
            let _ = app.autolaunch().enable();

            // Hide main window if launched via autostart
            let args: Vec<String> = std::env::args().collect();
            if args.iter().any(|arg| arg == "--autostart") {
                if let Some(main_win) = app.get_webview_window("main") {
                    let _ = main_win.hide();
                }
            }

            let app_handle = app.handle().clone();
            let state_for_hotkey = shared_state.clone();

            // ── System Tray ────────────────────────────────────────────────
            use tauri::tray::TrayIconBuilder;
            use tauri::menu::{Menu, MenuItem};

            if let (Ok(quit_i), Ok(show_i)) = (
                MenuItem::with_id(app, "quit", "Quit", true, None::<&str>),
                MenuItem::with_id(app, "show", "Settings", true, None::<&str>)
            ) {
                if let Ok(menu) = Menu::with_items(app, &[&show_i, &quit_i]) {
                    if let Some(icon) = app.default_window_icon() {
                        let _ = TrayIconBuilder::new()
                            .icon(icon.clone())
                            .menu(&menu)
                            .on_menu_event(|app, event| match event.id.as_ref() {
                                "quit" => app.exit(0),
                                "show" => {
                                    if let Some(win) = app.get_webview_window("main") {
                                        let _ = win.show();
                                        let _ = win.set_focus();
                                    }
                                }
                                _ => {}
                            })
                            .build(app);
                    }
                }
            }

            // ── Start IPC listener via Unix Domain Socket ────────────────
            // Listens for external signals (like "--toggle-recording")
            // Works securely on Wayland via custom system shortcuts.
            let app_handle_for_ipc = app_handle.clone();
            std::thread::spawn(move || {
                use std::io::Read;
                use std::os::unix::net::UnixListener;
                
                let socket_path = "/tmp/whispershell.sock";
                
                // Remove existing socket if it exists (from a crashed run)
                let _ = std::fs::remove_file(socket_path);
                
                match UnixListener::bind(socket_path) {
                    Ok(listener) => {
                        println!("[WhisperShell] ✅ IPC socket listener started at {}", socket_path);
                        for stream in listener.incoming() {
                            match stream {
                                Ok(mut stream) => {
                                    let mut buffer = String::new();
                                    if let Ok(_) = stream.read_to_string(&mut buffer) {
                                        let cmd = buffer.trim();
                                        if cmd == "TOGGLE" {
                                            let app_clone = app_handle_for_ipc.clone();
                                            let state_clone = state_for_hotkey.clone();
                                            std::thread::spawn(move || {
                                                handle_hotkey_press(&app_clone, state_clone);
                                            });
                                        } else if cmd == "TOGGLE_CONFIG" {
                                            let app_clone = app_handle_for_ipc.clone();
                                            std::thread::spawn(move || {
                                                if let Some(main_window) = app_clone.get_webview_window("main") {
                                                    if main_window.is_visible().unwrap_or(false) {
                                                        let _ = main_window.hide();
                                                    } else {
                                                        let _ = main_window.show();
                                                        let _ = main_window.set_focus();
                                                    }
                                                }
                                            });
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("[WhisperShell] Socket error: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[WhisperShell] ❌ Failed to bind IPC socket at {}: {}", socket_path, e);
                    }
                }
            });

            // ── Load Whisper model in background ──────────────────────────
            let model_path = resolve_model_path(&app_handle);
            let app_handle2     = app.handle().clone();
            let state_for_loader = shared_state.clone();

            std::thread::spawn(move || {
                println!("[WhisperShell] Loading model from: {}", model_path.display());
                match load_whisper_context(&model_path) {
                    Ok(ctx) => {
                        {
                            let mut s = state_for_loader.lock().unwrap();
                            s.whisper_ctx = Some(ctx);
                        }
                        println!("[WhisperShell] ✅ Model loaded and ready!");
                        let config = load_config(app_handle2.clone());
                        let model_key = config.get("model").and_then(|v| v.as_str()).unwrap_or("parakeet");
                        let _ = app_handle2.emit("model_ready", get_model_display_name(model_key));
                    }
                    Err(e) => {
                        eprintln!("[WhisperShell] ❌ Model load failed: {e}");
                        let _ = app_handle2.emit("whisper_error", format!("Model load failed: {e}"));
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_info, 
            get_status,
            select_directory,
            select_file,
            get_installed_models,
            load_config,
            save_config,
            log_to_terminal,
            download_model,
            cancel_download
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

fn prune_directory(dir: &PathBuf, limit: usize) {
    if limit == 0 { return; }
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut files: Vec<_> = entries.flatten()
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("wav"))
            .filter_map(|e| {
                e.metadata().ok().and_then(|m| m.modified().ok()).map(|time| (e.path(), time))
            })
            .collect();
        
        files.sort_by(|a, b| a.1.cmp(&b.1)); // oldest first
        
        if files.len() > limit {
            for (path, _) in files.iter().take(files.len() - limit) {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

fn prune_log_file(file: &PathBuf, limit: usize) {
    if limit == 0 { return; }
    if let Ok(content) = std::fs::read_to_string(file) {
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.len() > limit {
            let to_keep = &lines[lines.len() - limit..];
            if let Ok(mut f) = std::fs::File::create(file) {
                use std::io::Write;
                for line in to_keep {
                    let _ = writeln!(f, "{}", line);
                }
            }
        }
    }
}
