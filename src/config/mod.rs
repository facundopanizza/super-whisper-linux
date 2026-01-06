mod schema;

pub use schema::*;

use crate::error::{ConfigError, Result};
use std::path::PathBuf;
use tracing::info;

/// Get the configuration directory path
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("super-whisper-linux")
}

/// Get the data directory path
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("super-whisper-linux")
}

/// Get the default socket path
pub fn socket_path() -> PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
        .join("super-whisper.sock")
}

/// Get the default model path
pub fn default_model_path() -> PathBuf {
    data_dir().join("models").join("ggml-base.bin")
}

/// Load configuration from file or return defaults
pub fn load_config() -> Result<AppConfig> {
    let config_path = config_dir().join("config.toml");

    if config_path.exists() {
        info!("Loading configuration from {:?}", config_path);
        let content = std::fs::read_to_string(&config_path).map_err(ConfigError::ReadError)?;
        let config: AppConfig = toml::from_str(&content).map_err(ConfigError::ParseError)?;
        Ok(config)
    } else {
        info!("No configuration file found, using defaults");
        Ok(AppConfig::default())
    }
}

/// Save configuration to file
pub fn save_config(config: &AppConfig) -> Result<()> {
    let config_path = config_dir().join("config.toml");

    // Ensure directory exists
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(ConfigError::ReadError)?;
    }

    let content =
        toml::to_string_pretty(config).map_err(|e| ConfigError::ValidationError(e.to_string()))?;
    std::fs::write(&config_path, content).map_err(ConfigError::ReadError)?;

    info!("Configuration saved to {:?}", config_path);
    Ok(())
}

/// Initialize configuration directories
pub fn init_dirs() -> Result<()> {
    let dirs = [config_dir(), data_dir(), data_dir().join("models")];

    for dir in dirs {
        if !dir.exists() {
            std::fs::create_dir_all(&dir).map_err(ConfigError::ReadError)?;
            info!("Created directory {:?}", dir);
        }
    }

    Ok(())
}

impl AppConfig {
    /// Get the effective socket path
    pub fn socket_path(&self) -> PathBuf {
        self.hotkey
            .socket_path
            .clone()
            .unwrap_or_else(socket_path)
    }

    /// Get the effective model path for whisper
    pub fn model_path(&self) -> PathBuf {
        self.providers
            .whisper_local
            .model_path
            .clone()
            .unwrap_or_else(default_model_path)
    }

    /// Get the API key for OpenAI (config or env)
    pub fn openai_api_key(&self) -> Option<String> {
        self.providers
            .openai
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
    }

    /// Get the API key for Groq (config or env)
    pub fn groq_api_key(&self) -> Option<String> {
        self.providers
            .groq
            .api_key
            .clone()
            .or_else(|| std::env::var("GROQ_API_KEY").ok())
    }

    /// Get the API key for Deepgram (config or env)
    pub fn deepgram_api_key(&self) -> Option<String> {
        self.providers
            .deepgram
            .api_key
            .clone()
            .or_else(|| std::env::var("DEEPGRAM_API_KEY").ok())
    }
}
