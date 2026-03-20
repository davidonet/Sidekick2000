use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Context {
    pub id: String,
    pub label: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Speaker {
    pub name: String,
    pub organization: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub groq_api_key: String,
    #[serde(default)]
    pub anthropic_api_key: String,
    #[serde(default)]
    pub working_folder: String,
    #[serde(default)]
    pub github_repo: String,
    #[serde(default = "default_meetings_subfolder")]
    pub meetings_subfolder: String,
    #[serde(default = "default_language")]
    pub default_language: String,
    #[serde(default)]
    pub default_speakers: Vec<Speaker>,
    #[serde(default)]
    pub contexts: Vec<Context>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            groq_api_key: String::new(),
            anthropic_api_key: String::new(),
            working_folder: String::new(),
            github_repo: String::new(),
            meetings_subfolder: default_meetings_subfolder(),
            default_language: default_language(),
            default_speakers: Vec::new(),
            contexts: Vec::new(),
        }
    }
}

fn default_meetings_subfolder() -> String {
    "Meetings".to_string()
}

fn default_language() -> String {
    "fr".to_string()
}

pub fn settings_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".sidekick2000")
        .join("settings.json")
}

pub fn load() -> Settings {
    let path = settings_path();
    if !path.exists() {
        return Settings::default();
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

pub fn save(settings: &Settings) -> Result<()> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(settings)?;
    std::fs::write(&path, content)?;
    Ok(())
}
