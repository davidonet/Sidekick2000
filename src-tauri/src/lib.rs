mod audio;
mod diarize;
mod export;
mod github;
mod merge;
mod pipeline;
mod summarize;
mod transcribe;

use audio::AudioRecorder;
use pipeline::PipelineConfig;
use std::path::PathBuf;
use std::sync::Mutex;

/// Application state shared across commands
struct AppState {
    recorder: AudioRecorder,
    temp_dir: PathBuf,
}

#[tauri::command]
async fn start_recording(
    state: tauri::State<'_, Mutex<AppState>>,
    _app: tauri::AppHandle,
) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    state
        .recorder
        .start()
        .map_err(|e| format!("Failed to start recording: {}", e))
}

#[tauri::command]
async fn stop_recording(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(String, String), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let (ogg_path, wav_path) = state
        .recorder
        .stop(&state.temp_dir)
        .map_err(|e| format!("Failed to stop recording: {}", e))?;

    Ok((
        ogg_path.to_string_lossy().to_string(),
        wav_path.to_string_lossy().to_string(),
    ))
}

#[tauri::command]
async fn get_audio_level(state: tauri::State<'_, Mutex<AppState>>) -> Result<f32, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    Ok(state.recorder.current_level())
}

#[tauri::command]
async fn get_elapsed(state: tauri::State<'_, Mutex<AppState>>) -> Result<f64, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    Ok(state.recorder.elapsed_secs())
}

#[tauri::command]
async fn is_recording(state: tauri::State<'_, Mutex<AppState>>) -> Result<bool, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    Ok(state.recorder.is_recording())
}

#[tauri::command]
async fn run_pipeline(
    config: PipelineConfig,
    app: tauri::AppHandle,
) -> Result<pipeline::PipelineResult, String> {
    pipeline::run(config, app)
        .await
        .map_err(|e| format!("Pipeline failed: {}", e))
}

#[tauri::command]
fn get_default_output_dir() -> String {
    let home = dirs::document_dir().unwrap_or_else(|| PathBuf::from("."));
    let output_dir = home.join("MeetingScribe");
    output_dir.to_string_lossy().to_string()
}

#[tauri::command]
async fn open_file(path: String) -> Result<(), String> {
    open::that(&path).map_err(|e| format!("Failed to open file: {}", e))
}

#[tauri::command]
fn load_context_file(name: String) -> Result<String, String> {
    // Look for context files relative to the executable or in a known location
    let paths = vec![
        // Development: relative to project root
        PathBuf::from(format!("../contexts/{}", name)),
        // Bundled: next to executable
        std::env::current_exe()
            .unwrap_or_default()
            .parent()
            .unwrap_or(Path::new("."))
            .join("contexts")
            .join(&name),
        // Fallback: absolute path
        PathBuf::from(&name),
    ];

    for path in &paths {
        if path.exists() {
            return std::fs::read_to_string(path)
                .map_err(|e| format!("Failed to read context file: {}", e));
        }
    }

    // Return empty string if not found
    log::warn!("Context file not found: {}", name);
    Ok(String::new())
}

use std::path::Path;

pub fn run() {
    // Load .env file
    let _ = dotenvy::dotenv();
    env_logger::init();

    let temp_dir = std::env::temp_dir().join("meeting-scribe");
    let _ = std::fs::create_dir_all(&temp_dir);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(Mutex::new(AppState {
            recorder: AudioRecorder::new(),
            temp_dir,
        }))
        .invoke_handler(tauri::generate_handler![
            start_recording,
            stop_recording,
            get_audio_level,
            get_elapsed,
            is_recording,
            run_pipeline,
            get_default_output_dir,
            open_file,
            load_context_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
