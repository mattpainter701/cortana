use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CortanaConfig {
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub speech: SpeechConfig,
    #[serde(default)]
    pub summary: SummaryConfig,
    #[serde(default)]
    pub llm: LlmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_true")]
    pub show_recap_banner: bool,
    #[serde(default = "default_renderer")]
    pub renderer: String,
    #[serde(default = "default_face_frame_rate")]
    pub face_frame_rate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechConfig {
    #[serde(default = "default_tts_provider")]
    pub tts_provider: String,
    #[serde(default = "default_true")]
    pub enable_lip_sync: bool,
    #[serde(default)]
    pub speak_by_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryConfig {
    #[serde(default = "default_true")]
    pub allow_llm_deep_recaps: bool,
    #[serde(default = "default_banner_max_chars")]
    pub banner_max_chars: usize,
    #[serde(default = "default_speech_recap_sentences")]
    pub speech_recap_sentences: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_llm_provider")]
    pub provider: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_llm_endpoint")]
    pub endpoint: String,
    #[serde(default = "default_llm_model")]
    pub model: String,
}

impl Default for CortanaConfig {
    fn default() -> Self {
        Self {
            ui: UiConfig::default(),
            speech: SpeechConfig::default(),
            summary: SummaryConfig::default(),
            llm: LlmConfig::default(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_recap_banner: true,
            renderer: "ascii".into(),
            face_frame_rate: 15,
        }
    }
}

impl Default for SpeechConfig {
    fn default() -> Self {
        Self {
            tts_provider: "system".into(),
            enable_lip_sync: true,
            speak_by_default: false,
        }
    }
}

impl Default for SummaryConfig {
    fn default() -> Self {
        Self {
            allow_llm_deep_recaps: true,
            banner_max_chars: 120,
            speech_recap_sentences: 2,
        }
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        let api_key = std::env::var("DEEPSEEK_API_KEY")
            .or_else(|_| std::env::var("OPENAI_API_KEY"))
            .unwrap_or_default();
        Self {
            provider: "openai-compatible".into(),
            api_key,
            endpoint: "https://api.deepseek.com/v1".into(),
            model: "deepseek-chat".into(),
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_renderer() -> String {
    "canvas".into()
}
fn default_face_frame_rate() -> u32 {
    15
}
fn default_tts_provider() -> String {
    "system".into()
}
fn default_banner_max_chars() -> usize {
    120
}
fn default_speech_recap_sentences() -> usize {
    2
}
fn default_llm_provider() -> String {
    "openai-compatible".into()
}
fn default_llm_endpoint() -> String {
    "https://api.deepseek.com/v1".into()
}
fn default_llm_model() -> String {
    "deepseek-chat".into()
}

impl CortanaConfig {
    pub fn load() -> Self {
        // Load .env from project directory (simple loader, no dotenv dependency)
        Self::load_dotenv();

        if let Some(path) = config_path() {
            if path.exists() {
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        match toml::from_str(&content) {
                            Ok(config) => return config,
                            Err(e) => eprintln!("config parse error: {e}, using defaults"),
                        }
                    }
                    Err(e) => eprintln!("config read error: {e}, using defaults"),
                }
            }
        }
        Self::default()
    }

    /// Load KEY=VALUE pairs from .env in the current directory into the environment.
    fn load_dotenv() {
        let paths = [
            std::path::PathBuf::from(".env"),
            directories::BaseDirs::new()
                .map(|b| b.home_dir().join(".cortana.env"))
                .unwrap_or_default(),
        ];
        for path in &paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some((key, value)) = line.split_once('=') {
                        let value = value.trim().trim_matches('"').trim_matches('\'');
                        if std::env::var(key.trim()).is_err() {
                            std::env::set_var(key.trim(), value);
                        }
                    }
                }
            }
        }
    }

    pub fn save_default(path: &std::path::Path) -> Result<(), String> {
        let config = Self::default();
        let toml_str =
            toml::to_string_pretty(&config).map_err(|e| format!("serialize error: {e}"))?;
        std::fs::write(path, toml_str).map_err(|e| format!("write error: {e}"))?;
        Ok(())
    }
}

fn config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "cortana", "cortana")
        .map(|dirs| dirs.config_dir().join("config.toml"))
}
