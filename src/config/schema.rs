use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub audio: AudioConfig,
    pub hotkey: HotkeyConfig,
    pub tray: TrayConfig,
    pub providers: ProvidersConfig,
    pub logging: LoggingConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            audio: AudioConfig::default(),
            hotkey: HotkeyConfig::default(),
            tray: TrayConfig::default(),
            providers: ProvidersConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// Default STT provider
    pub default_provider: ProviderType,
    /// Language hint (ISO 639-1 code, or "auto")
    pub language: String,
    /// Enable audio feedback sounds
    pub audio_feedback: bool,
    /// Auto-paste after transcription
    pub auto_paste: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            default_provider: ProviderType::WhisperLocal,
            language: "auto".to_string(),
            audio_feedback: true,
            auto_paste: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderType {
    WhisperLocal,
    OpenAI,
    Groq,
    Deepgram,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::WhisperLocal => write!(f, "whisper-local"),
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Groq => write!(f, "groq"),
            ProviderType::Deepgram => write!(f, "deepgram"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioConfig {
    /// Input device name (empty = default)
    pub input_device: Option<String>,
    /// Sample rate (whisper requires 16000)
    pub sample_rate: u32,
    /// Silence detection threshold (0.0 - 1.0)
    pub silence_threshold: f32,
    /// Auto-stop after silence (seconds, 0 = disabled)
    pub silence_timeout: f32,
    /// Maximum recording duration (seconds)
    pub max_duration: u32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            input_device: None,
            sample_rate: 16000,
            silence_threshold: 0.01,
            silence_timeout: 2.0,
            max_duration: 300,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeyConfig {
    /// IPC socket path (default: $XDG_RUNTIME_DIR/super-whisper.sock)
    pub socket_path: Option<PathBuf>,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self { socket_path: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TrayConfig {
    /// Show in system tray
    pub enabled: bool,
    /// Icon theme (embedded, system, or path)
    pub icon_theme: String,
}

impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            icon_theme: "embedded".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProvidersConfig {
    #[serde(rename = "whisper-local")]
    pub whisper_local: WhisperLocalConfig,
    pub openai: OpenAIConfig,
    pub groq: GroqConfig,
    pub deepgram: DeepgramConfig,
}

impl Default for ProvidersConfig {
    fn default() -> Self {
        Self {
            whisper_local: WhisperLocalConfig::default(),
            openai: OpenAIConfig::default(),
            groq: GroqConfig::default(),
            deepgram: DeepgramConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WhisperLocalConfig {
    pub enabled: bool,
    /// Path to whisper.cpp model file
    pub model_path: Option<PathBuf>,
    /// Model variant: tiny, base, small, medium, large
    pub model: String,
    /// Use GPU acceleration if available
    pub use_gpu: bool,
    /// Number of threads (0 = auto)
    pub threads: u32,
}

impl Default for WhisperLocalConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model_path: None,
            model: "base".to_string(),  // multilingual model
            use_gpu: true,
            threads: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OpenAIConfig {
    pub enabled: bool,
    /// API key (or use OPENAI_API_KEY env var)
    pub api_key: Option<String>,
    /// Model name
    pub model: String,
    /// API endpoint
    pub endpoint: String,
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: None,
            model: "whisper-1".to_string(),
            endpoint: "https://api.openai.com/v1/audio/transcriptions".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GroqConfig {
    pub enabled: bool,
    /// API key (or use GROQ_API_KEY env var)
    pub api_key: Option<String>,
    /// Model name
    pub model: String,
    /// API endpoint
    pub endpoint: String,
}

impl Default for GroqConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: None,
            model: "whisper-large-v3".to_string(),
            endpoint: "https://api.groq.com/openai/v1/audio/transcriptions".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DeepgramConfig {
    pub enabled: bool,
    /// API key (or use DEEPGRAM_API_KEY env var)
    pub api_key: Option<String>,
    /// Model name
    pub model: String,
    /// Features to enable
    pub features: Vec<String>,
}

impl Default for DeepgramConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: None,
            model: "nova-2".to_string(),
            features: vec!["punctuate".to_string(), "smart_format".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Log level: trace, debug, info, warn, error
    pub level: String,
    /// Log to file
    pub file: Option<PathBuf>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file: None,
        }
    }
}
