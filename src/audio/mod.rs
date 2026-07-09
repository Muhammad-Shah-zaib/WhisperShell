use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use std::sync::{Arc, Mutex};

pub struct AudioRecorder {
    // Raw PCM samples
    buffer: Arc<Mutex<Vec<f32>>>,
    stream: Option<cpal::Stream>,
    sample_rate: u32,
    channels: usize,
}

impl AudioRecorder {
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device available")?;

        let config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get default input config: {e}"))?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        println!(
            "[WhisperShell] Audio device: {} Hz, {} channel(s)",
            sample_rate, channels
        );

        Ok(AudioRecorder {
            buffer: Arc::new(Mutex::new(Vec::new())),
            stream: None,
            sample_rate,
            channels,
        })
    }

    // Start capture
    pub fn start(&mut self) -> Result<(), String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device available")?;

        let config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get input config: {e}"))?;

        let buffer = self.buffer.clone();
        // Clear the buffer from any previous recording
        buffer.lock().unwrap().clear();

        let err_fn = |err| eprintln!("[WhisperShell] Audio stream error: {}", err);
        let stream_config: cpal::StreamConfig = config.clone().into();

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &_| {
                        buffer.lock().unwrap().extend_from_slice(data);
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("Failed to build input stream: {e}"))?,
            fmt => return Err(format!("Unsupported sample format: {:?}", fmt)),
        };

        stream.play().map_err(|e| format!("Failed to play stream: {e}"))?;
        self.stream = Some(stream);

        println!("[WhisperShell] 🔴 Recording started");
        Ok(())
    }

    // Stop capture and return 16kHz mono audio
    pub fn stop_and_get_audio(&mut self) -> Result<Vec<f32>, String> {
        if let Some(stream) = self.stream.take() {
            stream
                .pause()
                .map_err(|e| format!("Failed to pause stream: {e}"))?;
        }

        let raw: Vec<f32> = self.buffer.lock().unwrap().clone();
        if raw.is_empty() {
            return Err("No audio was recorded".into());
        }

        println!(
            "[WhisperShell] ⏹️  Recording stopped ({} raw samples)",
            raw.len()
        );

        // --- Stereo → Mono ---
        let mono: Vec<f32> = if self.channels > 1 {
            raw.chunks_exact(self.channels)
                .map(|chunk| chunk.iter().sum::<f32>() / self.channels as f32)
                .collect()
        } else {
            raw
        };

        // --- Resample to 16kHz if necessary ---
        if self.sample_rate == 16000 {
            return Ok(mono);
        }

        println!(
            "[WhisperShell] Resampling {} Hz → 16000 Hz ({} samples)...",
            self.sample_rate,
            mono.len()
        );

        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        let mut resampler = SincFixedIn::<f32>::new(
            16000.0 / self.sample_rate as f64,
            2.0,
            params,
            mono.len(),
            1,
        )
        .map_err(|e| format!("Failed to create resampler: {e}"))?;

        let input_waves = vec![mono];
        let mut resampled = resampler
            .process(&input_waves, None)
            .map_err(|e| format!("Resampling failed: {e}"))?;

        let output = resampled.pop().ok_or("Resampler returned no output")?;
        println!(
            "[WhisperShell] Resampled to {} samples at 16kHz",
            output.len()
        );

        Ok(output)
    }
}
