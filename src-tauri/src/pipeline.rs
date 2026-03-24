use crate::export;
use crate::export::ImageAnnotation;
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
    /// Git repo root folder for committing notes (empty = no commit)
    pub working_folder: String,
    pub meeting_name: String,
    /// OGG path for the local mic stream (always present).
    pub local_ogg_path: String,
    /// Speaker name for the local stream (from settings).
    pub local_speaker_name: String,
    /// OGG path for the remote audio stream (empty = no remote stream).
    #[serde(default)]
    pub remote_ogg_path: String,
    /// Speaker name for the remote stream (from settings).
    #[serde(default)]
    pub remote_speaker_name: String,
    /// Images pasted during recording (path + timecode).
    #[serde(default)]
    pub image_annotations: Vec<ImageAnnotation>,
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

/// Commit files to the working_folder git repo
fn git_commit_notes(working_folder: &str, notes_rel: &str, transcript_rel: &str, message: &str) {
    let add = std::process::Command::new("git")
        .current_dir(working_folder)
        .args(["add", notes_rel, transcript_rel])
        .output();

    if let Ok(o) = add {
        if o.status.success() {
            let _ = std::process::Command::new("git")
                .current_dir(working_folder)
                .args(["commit", "-m", message])
                .output();
        } else {
            log::warn!(
                "git add failed: {}",
                String::from_utf8_lossy(&o.stderr)
            );
        }
    }
}

/// Run the full processing pipeline
pub async fn run(
    config: PipelineConfig,
    groq_key: String,
    anthropic_key: String,
    together_key: String,
    summarization_provider: String,
    together_model: String,
    enable_summary: bool,
    enable_git_commit: bool,
    enable_github_issues: bool,
    app: AppHandle,
) -> Result<PipelineResult> {
    let local_ogg = PathBuf::from(&config.local_ogg_path);

    // Step 1: Transcribe both streams in parallel
    emit_progress(&app, "transcribing", 0.0);
    let language: Option<String> = if config.language_code.is_empty() {
        None
    } else {
        Some(config.language_code.clone())
    };

    let groq_key_local = groq_key.clone();
    let lang_local = language.clone();
    let local_handle = tokio::spawn(async move {
        transcribe::transcribe_with_groq(&local_ogg, lang_local.as_deref(), &groq_key_local).await
    });

    let remote_handle = if !config.remote_ogg_path.is_empty() {
        let remote_ogg = PathBuf::from(&config.remote_ogg_path);
        let groq_key_remote = groq_key.clone();
        let lang_remote = language.clone();
        Some(tokio::spawn(async move {
            transcribe::transcribe_with_groq(
                &remote_ogg,
                lang_remote.as_deref(),
                &groq_key_remote,
            )
            .await
        }))
    } else {
        None
    };

    let local_transcript = local_handle.await??;
    emit_progress(&app, "transcribing", 0.4);

    let remote_transcript = if let Some(handle) = remote_handle {
        Some(handle.await??)
    } else {
        None
    };
    emit_progress(&app, "merging", 0.5);

    // Step 2: Merge — speakers are already known, no diarization needed
    let merged = match remote_transcript {
        Some(remote) => merge::merge_dual_transcripts(
            &local_transcript.segments,
            &config.local_speaker_name,
            &remote.segments,
            &config.remote_speaker_name,
        ),
        None => local_transcript
            .segments
            .iter()
            .map(|s| merge::MergedSegment {
                speaker: config.local_speaker_name.clone(),
                start: s.start,
                end: s.end,
                text: s.text.trim().to_string(),
            })
            .collect(),
    };

    // Step 3: Generate transcript markdown (with pasted image markers)
    let transcript_md =
        export::export_transcript_markdown(&merged, &config.image_annotations);

    // Step 5: Summarize
    let notes = if enable_summary {
        emit_progress(&app, "summarizing", 0.60);

        let speaker_pairs: Vec<(String, String)> = config
            .speakers
            .iter()
            .map(|s| (s.name.clone(), s.organization.clone()))
            .collect();

        if summarization_provider == "together_ai" {
            summarize::summarize_with_together(
                &transcript_md,
                &config.context_content,
                &speaker_pairs,
                &config.language_name,
                &together_key,
                &together_model,
            )
            .await?
        } else {
            summarize::summarize_with_claude(
                &transcript_md,
                &config.context_content,
                &speaker_pairs,
                &config.language_name,
                &anthropic_key,
            )
            .await?
        }
    } else {
        transcript_md.clone()
    };

    // Step 6: Export
    emit_progress(&app, "exporting", 0.90);

    let output_dir = PathBuf::from(&config.output_dir);
    std::fs::create_dir_all(&output_dir)?;

    // File naming: YYYY-MM-DD_HHmm_Context_MeetingName.md
    let now = chrono::Local::now();
    let date = now.format("%Y-%m-%d").to_string();
    let time = now.format("%H%M").to_string();
    let context_sanitized = export::sanitize_filename(&config.context);
    let meeting_name_sanitized = export::sanitize_filename(&config.meeting_name);
    let base_name = match (context_sanitized.is_empty(), meeting_name_sanitized.is_empty()) {
        (true, true) => format!("{}_{}_meeting", date, time),
        (false, true) => format!("{}_{}_{}", date, time, context_sanitized),
        (true, false) => format!("{}_{}_{}", date, time, meeting_name_sanitized),
        (false, false) => format!("{}_{}_{}_{}", date, time, context_sanitized, meeting_name_sanitized),
    };

    let filename = format!("{}.md", base_name);
    let transcript_filename = format!("{}_transcript.md", base_name);

    let output_path = output_dir.join(&filename);
    let transcript_path = output_dir.join(&transcript_filename);

    std::fs::write(&output_path, &notes)?;
    log::info!("Meeting notes saved to: {}", output_path.display());

    std::fs::write(&transcript_path, &transcript_md)?;

    // Copy pasted images to the output dir and append a Screenshots section to both files
    if !config.image_annotations.is_empty() {
        let mut screenshots_md = String::from("\n\n---\n\n## Screenshots\n\n");
        for img in &config.image_annotations {
            let src = std::path::Path::new(&img.path);
            if let Some(basename) = src.file_name() {
                let dest = output_dir.join(basename);
                if src.exists() {
                    let _ = std::fs::copy(src, &dest);
                }
                let ts = export::format_timestamp(img.timecode_secs);
                screenshots_md.push_str(&format!(
                    "### {}\n\n![Screenshot at {}](./{}) \n\n",
                    ts,
                    ts,
                    basename.to_string_lossy()
                ));
            }
        }
        // Append to notes file
        let mut notes_content = std::fs::read_to_string(&output_path).unwrap_or_default();
        notes_content.push_str(&screenshots_md);
        std::fs::write(&output_path, &notes_content)?;
    }

    // Step 7: Git commit (if enabled and working_folder is set)
    if enable_git_commit && !config.working_folder.is_empty() {
        emit_progress(&app, "committing", 0.93);
        let working_folder = &config.working_folder;

        // Compute paths relative to the git root
        let notes_rel = output_path
            .strip_prefix(working_folder)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| output_path.to_string_lossy().to_string());
        let transcript_rel = transcript_path
            .strip_prefix(working_folder)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| transcript_path.to_string_lossy().to_string());

        let commit_label = if !meeting_name_sanitized.is_empty() {
            meeting_name_sanitized.clone()
        } else if !context_sanitized.is_empty() {
            context_sanitized.clone()
        } else {
            "general".to_string()
        };
        let commit_msg = format!("meeting: {} {}", commit_label, date);

        git_commit_notes(working_folder, &notes_rel, &transcript_rel, &commit_msg);
        log::info!("Git commit done: {}", commit_msg);
    }

    // Step 8: Create GitHub issues from action items (optional)
    let mut created_issues = Vec::new();
    if enable_github_issues && !config.github_repo.is_empty() {
        emit_progress(&app, "creating_issues", 0.95);

        let action_items = github::parse_action_items(&notes);
        log::info!(
            "Found {} action items to create as issues",
            action_items.len()
        );

        if !action_items.is_empty() {
            let issue_label = if config.meeting_name.is_empty() {
                &config.context
            } else {
                &config.meeting_name
            };
            created_issues = github::create_issues(
                &config.github_repo,
                &action_items,
                issue_label,
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
