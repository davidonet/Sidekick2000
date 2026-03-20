use crate::diarize;
use crate::export;
use crate::github;
use crate::merge;
use crate::summarize;
use crate::transcribe;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Speaker {
    pub name: String,
    pub organization: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub context: String,
    pub context_content: String,
    pub speakers: Vec<Speaker>,
    /// ISO 639-1 code for Groq transcription (e.g. "fr", "en")
    pub language_code: String,
    /// Full language name for Claude summarization (e.g. "French", "English")
    pub language_name: String,
    /// Optional GitHub repo (e.g. "owner/repo") to create issues for action items
    pub github_repo: String,
    pub output_dir: String,
    pub ogg_path: String,
    pub wav_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub notes_path: String,
    pub created_issues: Vec<github::CreatedIssue>,
}

#[derive(Clone, Serialize)]
pub struct PipelineProgress {
    pub step: String,
    pub progress: f64,
}

fn emit_progress(app: &AppHandle, step: &str, progress: f64) {
    let _ = app.emit(
        "pipeline-progress",
        PipelineProgress {
            step: step.to_string(),
            progress,
        },
    );
}

/// Run the full processing pipeline
pub async fn run(config: PipelineConfig, app: AppHandle) -> Result<PipelineResult> {
    let groq_key = std::env::var("GROQ_API_KEY")
        .map_err(|_| anyhow::anyhow!("GROQ_API_KEY not set. Add it to your .env file."))?;
    let anthropic_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set. Add it to your .env file."))?;

    let ogg_path = PathBuf::from(&config.ogg_path);
    let wav_path = PathBuf::from(&config.wav_path);

    // Step 1: Transcribe (async API call)
    emit_progress(&app, "transcribing", 0.0);
    let language: Option<String> = if config.language_code.is_empty() {
        None
    } else {
        Some(config.language_code.clone())
    };

    // Run transcription and diarization concurrently
    let groq_key_clone = groq_key.clone();
    let ogg_path_clone = ogg_path.clone();
    let wav_path_clone = wav_path.clone();
    let max_speakers = config.speakers.len().max(2);

    let transcript_handle = tokio::spawn(async move {
        transcribe::transcribe_with_groq(
            &ogg_path_clone,
            language.as_deref(),
            &groq_key_clone,
        )
        .await
    });

    emit_progress(&app, "diarizing", 0.15);
    let diarize_handle = tokio::task::spawn_blocking(move || {
        diarize::diarize(&wav_path_clone, 1, max_speakers)
    });

    let transcript = transcript_handle.await??;
    emit_progress(&app, "diarizing", 0.30);

    let diarization = diarize_handle.await??;
    emit_progress(&app, "merging", 0.50);

    // Step 3: Merge
    let merged = merge::merge(&transcript.segments, &diarization);

    // Step 4: Generate transcript markdown
    let transcript_md = export::export_transcript_markdown(&merged);

    // Step 5: Summarize with Claude
    emit_progress(&app, "summarizing", 0.60);

    let speaker_pairs: Vec<(String, String)> = config
        .speakers
        .iter()
        .map(|s| (s.name.clone(), s.organization.clone()))
        .collect();

    let notes = summarize::summarize_with_claude(
        &transcript_md,
        &config.context_content,
        &speaker_pairs,
        &config.language_name,
        &anthropic_key,
    )
    .await?;

    // Step 6: Export
    emit_progress(&app, "exporting", 0.90);

    let output_dir = PathBuf::from(&config.output_dir);
    std::fs::create_dir_all(&output_dir)?;

    // File naming: YYYY-MM-DD_Context.md
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let context_sanitized = export::sanitize_filename(&config.context);
    let filename = if context_sanitized.is_empty() {
        format!("{}_meeting.md", date)
    } else {
        format!("{}_{}.md", date, context_sanitized)
    };

    let output_path = output_dir.join(&filename);

    // Write the summary as the main output file
    std::fs::write(&output_path, &notes)?;
    log::info!("Meeting notes saved to: {}", output_path.display());

    // Also save the transcript
    let transcript_path = output_dir.join(format!(
        "{}_{}_transcript.md",
        date, context_sanitized
    ));
    std::fs::write(&transcript_path, &transcript_md)?;

    // Step 7: Create GitHub issues from action items (optional)
    let mut created_issues = Vec::new();
    if !config.github_repo.is_empty() {
        emit_progress(&app, "creating_issues", 0.95);

        let action_items = github::parse_action_items(&notes);
        log::info!(
            "Found {} action items to create as issues",
            action_items.len()
        );

        if !action_items.is_empty() {
            created_issues = github::create_issues(
                &config.github_repo,
                &action_items,
                &config.context,
                &date,
                &output_path.to_string_lossy(),
            );
        }
    }

    emit_progress(&app, "done", 1.0);

    Ok(PipelineResult {
        notes_path: output_path.to_string_lossy().to_string(),
        created_issues,
    })
}
