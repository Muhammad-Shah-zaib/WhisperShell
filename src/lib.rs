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
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, ShortcutState};
use whisper_rs::WhisperContext;

// App state

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

// Tauri commands

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

// Helper

fn resolve_model_path() -> PathBuf {
    // The binary runs from the workspace root; models live in tests/models/
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()                     // WhisperShell/
        .expect("CARGO_MANIFEST_DIR has no parent")
        .join("tests")
        .join("models")
        .join(MODEL_NAME)
}

// Hotkey toggle logic

fn handle_hotkey_press(app: &AppHandle, shared_state: Arc<Mutex<AppState>>) {
    let current_state = {
        let s = shared_state.lock().unwrap();
        s.recording_state.clone()
    };

    println!("[WhisperShell] 🔔 handle_hotkey_press called, current state: {:?}", current_state);

    match current_state {
        RecordingState::Idle => {
            println!("[WhisperShell] ▶️  Starting recording...");
            // Start recording
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
        }

        RecordingState::Recording => {
            println!("[WhisperShell] ⏹️  Stopping recording — starting transcription...");
            // Stop recording, process, transcribe
            let audio = {
                let mut s = shared_state.lock().unwrap();
                s.recording_state = RecordingState::Processing;
                let _ = app.emit("state_changed", "Processing");

                match s.recorder.as_mut() {
                    Some(rec) => rec.stop_and_get_audio(),
                    None => Err("No active recorder".into()),
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

            // Transcribe (this is CPU/GPU intensive — runs synchronously here since we're
            // already on a background thread spawned by the shortcut handler)
            let transcript = {
                let s = shared_state.lock().unwrap();
                match &s.whisper_ctx {
                    Some(ctx) => transcribe(ctx, &audio),
                    None => Err("Whisper model not loaded".into()),
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
                }
                Err(e) => {
                    eprintln!("[WhisperShell] Transcription error: {e}");
                    let _ = app.emit("whisper_error", e);
                }
            }

            s.recording_state = RecordingState::Idle;
            let _ = app.emit("state_changed", "Idle");
        }

        RecordingState::Processing => {
            // Ignore hotkey while already processing
            println!("[WhisperShell] Still processing, ignoring hotkey");
        }
    }
}

// Tauri entry point

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shared_state = Arc::new(Mutex::new(AppState::new()));
    let shared_state_for_setup = shared_state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    // Log every raw shortcut event for debugging
                    println!(
                        "[WhisperShell] 🎹 Raw shortcut event: key={:?} mods={:?} state={:?}",
                        shortcut.key, shortcut.mods, event.state()
                    );

                    // Only act on key-press (not release)
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }

                    // Check it's our registered shortcut: Ctrl+Space
                    let is_our_shortcut = shortcut.mods.contains(Modifiers::CONTROL)
                        && shortcut.key == Code::Space;

                    if is_our_shortcut {
                        println!("[WhisperShell] ✅ Hotkey matched! Dispatching toggle...");
                        let app_clone = app.clone();
                        let state_clone = shared_state_for_setup.clone();
                        // Spawn a new thread so we don't block the shortcut handler
                        std::thread::spawn(move || {
                            handle_hotkey_press(&app_clone, state_clone);
                        });
                    } else {
                        println!("[WhisperShell] ⚠️  Shortcut event fired but did not match Ctrl+Space");
                    }
                })
                .build(),
        )
        .manage(shared_state.clone())
        .setup(move |app| {
            // --- Register global shortcut: Ctrl + Space ---
            app.global_shortcut()
                .register(
                    tauri_plugin_global_shortcut::Shortcut::new(
                        Some(Modifiers::CONTROL),
                        Code::Space,
                    ),
                )
                .map_err(|e| format!("Failed to register global shortcut: {e}"))?;

            println!("[WhisperShell] ✅ Global shortcut registered: {HOTKEY_DISPLAY}");

            // --- Load Whisper model in a background thread so the window opens fast ---
            let model_path = resolve_model_path();
            let app_handle = app.handle().clone();
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
                        let _ = app_handle.emit("model_ready", MODEL_DISPLAY_NAME);
                    }
                    Err(e) => {
                        eprintln!("[WhisperShell] ❌ Model load failed: {e}");
                        let _ = app_handle.emit("whisper_error", format!("Model load failed: {e}"));
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_app_info, get_status])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
