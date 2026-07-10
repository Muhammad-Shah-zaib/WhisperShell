mod audio;
mod constants;
mod inference;
mod injection;

use audio::AudioRecorder;
use constants::{HOTKEY_DISPLAY, MODEL_DISPLAY_NAME, MODEL_NAME};
use inference::{load_whisper_context, transcribe};
use injection::copy_to_clipboard;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
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
}

impl AppState {
    fn new() -> Self {
        AppState {
            recording_state: RecordingState::Idle,
            recorder: None,
            whisper_ctx: None,
            last_transcript: String::new(),
        }
    }
}

// Tauri requires state types to be Send + Sync
unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

// ── Tauri commands ─────────────────────────────────────────────────────────

#[tauri::command]
fn get_app_info() -> serde_json::Value {
    serde_json::json!({
        "model": MODEL_DISPLAY_NAME,
        "hotkey": HOTKEY_DISPLAY,
    })
}

#[tauri::command]
fn get_status(state: tauri::State<Arc<Mutex<AppState>>>) -> serde_json::Value {
    let s = state.lock().unwrap();
    serde_json::json!({
        "state": format!("{:?}", s.recording_state),
        "transcript": s.last_transcript,
    })
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn resolve_model_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("CARGO_MANIFEST_DIR has no parent")
        .join("tests")
        .join("models")
        .join(MODEL_NAME)
}

// ── Overlay helpers ────────────────────────────────────────────────────────

fn show_overlay(app: &AppHandle) {
    if let Some(overlay) = app.get_webview_window("overlay") {
        // Re-assert always-on-top on every show
        let _ = overlay.set_always_on_top(true);

        // Pure physical-pixel math — works correctly on all scale factors and
        // multi-monitor setups. current_monitor() may return None if the window
        // has never been shown yet; fall back to primary monitor.
        let monitor_result = overlay
            .current_monitor()
            .ok()
            .flatten()
            .or_else(|| app.primary_monitor().ok().flatten());

        if let Some(monitor) = monitor_result {
            let scale   = monitor.scale_factor();
            let mon_pos = monitor.position();
            let mon_sz  = monitor.size();

            // Window logical size (from tauri.conf.json): 320 × 80
            let win_w  = (320.0 * scale) as u32;
            let win_h  = (80.0  * scale) as u32;
            let margin = (20.0  * scale) as u32;

            let x = mon_pos.x + ((mon_sz.width.saturating_sub(win_w)) / 2) as i32;
            let y = mon_pos.y + (mon_sz.height.saturating_sub(win_h + margin)) as i32;

            let _ = overlay.set_position(PhysicalPosition::new(x, y));
        }

        let _ = overlay.show();
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
            if let Err(e) = recorder.start() {
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

            let mut s = shared_state.lock().unwrap();
            match transcript {
                Ok(text) => {
                    println!("[WhisperShell] Transcript: {text}");
                    if let Err(e) = copy_to_clipboard(&text) {
                        eprintln!("[WhisperShell] Clipboard error: {e}");
                    }
                    s.last_transcript = text.clone();
                    let _ = app.emit("transcript_ready", text);

                    // Auto-dismiss overlay after 2 s
                    let app_clone = app.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_secs(2));
                        hide_overlay(&app_clone);
                    });
                }
                Err(e) => {
                    eprintln!("[WhisperShell] Transcription error: {e}");
                    let _ = app.emit("whisper_error", e);
                    hide_overlay(app);
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
        .manage(shared_state.clone())
        .setup(move |app| {
            let app_handle = app.handle().clone();
            let state_for_hotkey = shared_state.clone();

            // ── Start IPC listener via Unix Domain Socket ────────────────
            // Listens for external signals (like "--toggle-recording")
            // Works securely on Wayland via custom system shortcuts.
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
                                        if buffer == "TOGGLE" {
                                            let app_clone = app_handle.clone();
                                            let state_clone = state_for_hotkey.clone();
                                            std::thread::spawn(move || {
                                                handle_hotkey_press(&app_clone, state_clone);
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
            let model_path = resolve_model_path();
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
                        let _ = app_handle2.emit("model_ready", MODEL_DISPLAY_NAME);
                    }
                    Err(e) => {
                        eprintln!("[WhisperShell] ❌ Model load failed: {e}");
                        let _ = app_handle2.emit("whisper_error", format!("Model load failed: {e}"));
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_app_info, get_status])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
