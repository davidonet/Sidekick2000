use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub id: usize,
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptResult {
    pub text: String,
    pub segments: Vec<TranscriptSegment>,
}

/// Groq Whisper API response structures
#[derive(Debug, Deserialize)]
struct GroqResponse {
    text: String,
    segments: Option<Vec<GroqSegment>>,
}

#[derive(Debug, Deserialize)]
struct GroqSegment {
    start: f64,
    end: f64,
    text: String,
}

/// Transcribe audio using the Groq Whisper API
pub async fn transcribe_with_groq(
    audio_path: &Path,
    language: Option<&str>,
    api_key: &str,
) -> Result<TranscriptResult> {
    let file_bytes = std::fs::read(audio_path)
        .with_context(|| format!("Failed to read audio file: {}", audio_path.display()))?;

    let file_size_mb = file_bytes.len() as f64 / (1024.0 * 1024.0);
    log::info!(
        "Transcribing with Groq (whisper-large-v3-turbo): {} ({:.1} MB)",
        audio_path.display(),
        file_size_mb
    );

    if file_size_mb > 25.0 {
        log::warn!("File exceeds 25 MB free-tier limit, upload may fail");
    }

    let filename = audio_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Determine MIME type from extension
    let mime_type = match audio_path.extension().and_then(|e| e.to_str()) {
        Some("ogg") => "audio/ogg",
        Some("wav") => "audio/wav",
        Some("mp3") => "audio/mpeg",
        Some("flac") => "audio/flac",
        _ => "audio/wav",
    };

    let file_part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(filename)
        .mime_str(mime_type)?;

    let mut form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", "whisper-large-v3-turbo")
        .text("response_format", "verbose_json")
        .text("timestamp_granularities[]", "segment");

    if let Some(lang) = language {
        form = form.text("language", lang.to_string());
    }

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.groq.com/openai/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .context("Failed to send request to Groq API")?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        anyhow::bail!("Groq API error ({}): {}", status, error_text);
    }

    let groq_response: GroqResponse = response
        .json()
        .await
        .context("Failed to parse Groq response")?;

    let segments = groq_response
        .segments
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(i, seg)| TranscriptSegment {
            id: i,
            start: seg.start,
            end: seg.end,
            text: seg.text.trim().to_string(),
        })
        .collect();

    log::info!("Groq transcription completed successfully");

    Ok(TranscriptResult {
        text: groq_response.text,
        segments,
    })
}
