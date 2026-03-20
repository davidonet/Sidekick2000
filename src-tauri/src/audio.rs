use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use hound::{WavSpec, WavWriter};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Manages audio recording from the default input device.
///
/// Because `cpal::Stream` is not `Send` on macOS, we store the stream
/// behind an `Arc<Mutex<Option<...>>>` and keep it on the creating thread.
/// The stream's data callback pushes samples into a shared buffer.
pub struct AudioRecorder {
    is_recording: Arc<AtomicBool>,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: Arc<Mutex<u32>>,
    channels: Arc<Mutex<u16>>,
    start_time: Arc<Mutex<Option<Instant>>>,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            is_recording: Arc::new(AtomicBool::new(false)),
            samples: Arc::new(Mutex::new(Vec::new())),
            sample_rate: Arc::new(Mutex::new(44100)),
            channels: Arc::new(Mutex::new(1)),
            start_time: Arc::new(Mutex::new(None)),
        }
    }

    /// Start recording from the default input device.
    /// The stream is kept alive by a dedicated non-Send thread
    /// spawned via `dispatch` on macOS.
    pub fn start(&self) -> Result<()> {
        if self.is_recording.load(Ordering::SeqCst) {
            anyhow::bail!("Already recording");
        }

        // Clear previous samples
        self.samples.lock().unwrap().clear();

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("No input device available")?;

        log::info!("Recording from: {}", device.name().unwrap_or_default());

        let config = device.default_input_config()?;
        let sample_format = config.sample_format();
        let config: cpal::StreamConfig = config.into();

        *self.sample_rate.lock().unwrap() = config.sample_rate.0;
        *self.channels.lock().unwrap() = config.channels;
        *self.start_time.lock().unwrap() = Some(Instant::now());

        let is_recording = self.is_recording.clone();
        let is_recording2 = self.is_recording.clone();
        let samples = self.samples.clone();

        let err_fn = |err: cpal::StreamError| {
            log::error!("Stream error: {}", err);
        };

        let stream = match sample_format {
            SampleFormat::F32 => device.build_input_stream(
                &config,
                move |data: &[f32], _: &_| {
                    if !is_recording.load(Ordering::SeqCst) {
                        return;
                    }
                    samples.lock().unwrap().extend_from_slice(data);
                },
                err_fn,
                None,
            )?,
            SampleFormat::I16 => {
                let is_recording_i16 = self.is_recording.clone();
                let samples_i16 = self.samples.clone();
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _: &_| {
                        if !is_recording_i16.load(Ordering::SeqCst) {
                            return;
                        }
                        let floats: Vec<f32> =
                            data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                        samples_i16.lock().unwrap().extend_from_slice(&floats);
                    },
                    err_fn,
                    None,
                )?
            }
            format => anyhow::bail!("Unsupported sample format: {:?}", format),
        };

        stream.play()?;
        self.is_recording.store(true, Ordering::SeqCst);

        // Keep the stream alive by sending it via a channel to a dedicated thread.
        // We wrap it in StreamHolder which implements Send (unsafe but correct
        // because only one thread accesses it).
        let holder = StreamHolder(stream);
        std::thread::Builder::new()
            .name("audio-stream-keeper".to_string())
            .spawn(move || {
                let _stream = holder; // Keep stream alive
                while is_recording2.load(Ordering::SeqCst) {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                // Stream drops here, stopping recording
            })?;

        Ok(())
    }

    /// Stop recording and save the audio to WAV files
    pub fn stop(&self, output_dir: &PathBuf) -> Result<(PathBuf, PathBuf)> {
        self.is_recording.store(false, Ordering::SeqCst);

        // Give the stream thread time to notice and drop
        std::thread::sleep(std::time::Duration::from_millis(200));

        let samples = self.samples.lock().unwrap().clone();
        let native_sr = *self.sample_rate.lock().unwrap();
        let channels = *self.channels.lock().unwrap();

        if samples.is_empty() {
            anyhow::bail!("No audio recorded");
        }

        log::info!(
            "Recorded {} samples at {}Hz, {} channels",
            samples.len(),
            native_sr,
            channels
        );

        // Convert to mono if multi-channel
        let mono_samples = if channels > 1 {
            samples
                .chunks(channels as usize)
                .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
                .collect::<Vec<f32>>()
        } else {
            samples
        };

        // Resample to 16kHz for Whisper/diarization
        let target_sr = 16000u32;
        let resampled = resample(&mono_samples, native_sr, target_sr);

        std::fs::create_dir_all(output_dir)?;

        // Save WAV (for diarization — needs raw PCM)
        let wav_path = output_dir.join("recording.wav");
        save_wav(&resampled, target_sr, &wav_path)?;
        log::info!("Saved WAV: {}", wav_path.display());

        // Convert WAV → OGG/Opus via ffmpeg for Groq upload (much smaller)
        let ogg_path = output_dir.join("recording.ogg");
        convert_wav_to_ogg(&wav_path, &ogg_path)?;

        let ogg_size_mb = std::fs::metadata(&ogg_path)
            .map(|m| m.len() as f64 / (1024.0 * 1024.0))
            .unwrap_or(0.0);
        let wav_size_mb = std::fs::metadata(&wav_path)
            .map(|m| m.len() as f64 / (1024.0 * 1024.0))
            .unwrap_or(0.0);
        log::info!(
            "Audio sizes — WAV: {:.1} MB, OGG/Opus: {:.1} MB (compression ratio: {:.0}x)",
            wav_size_mb,
            ogg_size_mb,
            if ogg_size_mb > 0.0 { wav_size_mb / ogg_size_mb } else { 0.0 }
        );

        Ok((ogg_path, wav_path))
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }

    pub fn elapsed_secs(&self) -> f64 {
        self.start_time
            .lock()
            .unwrap()
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0)
    }

    /// Get the current RMS level from accumulated samples
    pub fn current_level(&self) -> f32 {
        let samples = self.samples.lock().unwrap();
        if samples.len() < 1600 {
            return 0.0;
        }
        let recent = &samples[samples.len().saturating_sub(4800)..];
        compute_rms(recent)
    }
}

/// Wrapper to allow cpal::Stream to be sent across threads.
/// SAFETY: We ensure single-thread access via the keeper thread pattern.
/// The field is intentionally "unused" — its purpose is to keep the Stream alive.
#[allow(dead_code)]
struct StreamHolder(cpal::Stream);
unsafe impl Send for StreamHolder {}

fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

/// Simple linear resampling
fn resample(samples: &[f32], from_sr: u32, to_sr: u32) -> Vec<f32> {
    if from_sr == to_sr {
        return samples.to_vec();
    }

    let ratio = from_sr as f64 / to_sr as f64;
    let new_len = (samples.len() as f64 / ratio) as usize;
    let mut result = Vec::with_capacity(new_len);

    for i in 0..new_len {
        let src_idx = i as f64 * ratio;
        let idx = src_idx as usize;
        let frac = (src_idx - idx as f64) as f32;

        let sample = if idx + 1 < samples.len() {
            samples[idx] * (1.0 - frac) + samples[idx + 1] * frac
        } else if idx < samples.len() {
            samples[idx]
        } else {
            0.0
        };

        result.push(sample);
    }

    result
}

/// Convert WAV to OGG/Opus using ffmpeg for compact upload to Groq.
/// Opus at 32kbps mono gives ~15x compression over 16-bit PCM WAV.
fn convert_wav_to_ogg(wav_path: &PathBuf, ogg_path: &PathBuf) -> Result<()> {
    log::info!("Converting WAV → OGG/Opus via ffmpeg");

    let output = std::process::Command::new("ffmpeg")
        .args([
            "-y",               // overwrite output
            "-i",
            wav_path.to_str().unwrap_or_default(),
            "-c:a", "libopus",  // Opus codec
            "-b:a", "32k",      // 32 kbps — excellent quality for speech
            "-ac", "1",         // mono
            "-ar", "16000",     // 16 kHz
            "-application", "voip", // optimized for speech
            ogg_path.to_str().unwrap_or_default(),
        ])
        .output()
        .context("Failed to run ffmpeg. Is ffmpeg installed? (brew install ffmpeg)")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ffmpeg encoding failed: {}", stderr);
    }

    log::info!("OGG/Opus encoding complete: {}", ogg_path.display());
    Ok(())
}

/// Save samples as a 16-bit PCM WAV file
fn save_wav(samples: &[f32], sample_rate: u32, path: &PathBuf) -> Result<()> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec)?;

    for &sample in samples {
        let s = (sample * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        writer.write_sample(s)?;
    }

    writer.finalize()?;
    Ok(())
}
