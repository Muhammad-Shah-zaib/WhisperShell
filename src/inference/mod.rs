use std::path::PathBuf;
use whisper_rs::{WhisperContext, WhisperContextParameters};

// Detect NVIDIA Vulkan device index
pub fn detect_nvidia_vulkan_device() -> i32 {
    let icd_dirs = ["/usr/share/vulkan/icd.d", "/etc/vulkan/icd.d"];

    let mut has_intel = false;
    let mut has_nvidia = false;

    for dir in &icd_dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry
                    .file_name()
                    .into_string()
                    .unwrap_or_default()
                    .to_lowercase();
                if name.contains("intel") && !name.contains("nouveau") {
                    has_intel = true;
                }
                if name.contains("nvidia") {
                    has_nvidia = true;
                }
            }
        }
    }

    if has_intel && has_nvidia {
        println!("[WhisperShell] Hybrid GPU detected (Intel iGPU + NVIDIA dGPU)");
        println!("[WhisperShell] → Targeting Vulkan device 1 (NVIDIA discrete GPU)");
        1
    } else if has_nvidia {
        println!("[WhisperShell] NVIDIA-only GPU detected → Vulkan device 0");
        0
    } else {
        println!("[WhisperShell] No NVIDIA ICD found → defaulting to Vulkan device 0");
        0
    }
}

// Load Whisper model
pub fn load_whisper_context(model_path: &PathBuf) -> Result<WhisperContext, String> {
    let gpu_device = detect_nvidia_vulkan_device();

    let mut ctx_params = WhisperContextParameters::default();
    ctx_params.gpu_device(gpu_device);
    ctx_params.flash_attn(true);

    println!(
        "[WhisperShell] Loading model: {}",
        model_path.display()
    );
    println!(
        "[WhisperShell]   use_gpu    = {}",
        ctx_params.use_gpu
    );
    println!("[WhisperShell]   gpu_device = {}", gpu_device);
    println!(
        "[WhisperShell]   flash_attn = {}",
        ctx_params.flash_attn
    );

    let path_str = model_path
        .to_str()
        .ok_or("Model path contains invalid UTF-8")?;

    WhisperContext::new_with_params(path_str, ctx_params)
        .map_err(|e| format!("Failed to load Whisper model: {e}"))
}

// Run transcription
pub fn transcribe(ctx: &WhisperContext, audio: &[f32]) -> Result<String, String> {
    let mut state = ctx
        .create_state()
        .map_err(|e| format!("Failed to create Whisper state: {e}"))?;

    let mut params =
        whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
    params.set_print_progress(false);
    params.set_print_special(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    state
        .full(params, audio)
        .map_err(|e| format!("Whisper inference failed: {e}"))?;

    // Use the same segment API that the validation test proved works
    let num_segments = state.full_n_segments();
    let mut transcript = String::new();

    for i in 0..num_segments {
        if let Some(segment) = state.get_segment(i) {
            if let Ok(text) = segment.to_str() {
                transcript.push_str(text);
            }
        }
    }

    Ok(transcript.trim().to_string())
}
