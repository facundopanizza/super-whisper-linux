use thiserror::Error;

/// Main application error type
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Audio error: {0}")]
    Audio(#[from] AudioError),

    #[error("STT error: {0}")]
    Stt(#[from] SttError),

    #[error("IPC error: {0}")]
    Ipc(#[from] IpcError),

    #[error("Tray error: {0}")]
    Tray(#[from] TrayError),

    #[error("Clipboard error: {0}")]
    Clipboard(#[from] ClipboardError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Application error: {0}")]
    Other(String),
}

/// Configuration-related errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse config: {0}")]
    ParseError(#[from] toml::de::Error),

    #[error("Invalid configuration: {0}")]
    ValidationError(String),

    #[error("Missing required field: {0}")]
    MissingField(String),
}

/// Audio capture and processing errors
#[derive(Error, Debug)]
pub enum AudioError {
    #[error("No audio input device found")]
    NoInputDevice,

    #[error("Failed to get audio device: {0}")]
    DeviceError(String),

    #[error("Failed to get audio config: {0}")]
    ConfigError(String),

    #[error("Failed to build audio stream: {0}")]
    StreamError(String),

    #[error("Audio capture error: {0}")]
    CaptureError(String),

    #[error("Sample rate conversion error: {0}")]
    ResampleError(String),

    #[error("WAV encoding error: {0}")]
    WavError(String),
}

/// Speech-to-text provider errors
#[derive(Error, Debug)]
pub enum SttError {
    #[error("Model loading error: {0}")]
    ModelError(String),

    #[error("Transcription failed: {0}")]
    TranscriptionError(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Invalid audio format: {0}")]
    InvalidAudio(String),

    #[error("Provider not available: {0}")]
    ProviderUnavailable(String),
}

/// IPC communication errors
#[derive(Error, Debug)]
pub enum IpcError {
    #[error("Socket error: {0}")]
    SocketError(#[from] std::io::Error),

    #[error("Failed to send command: {0}")]
    SendError(String),

    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    #[error("Connection refused")]
    ConnectionRefused,
}

/// System tray errors
#[derive(Error, Debug)]
pub enum TrayError {
    #[error("Failed to create tray: {0}")]
    CreateError(String),

    #[error("D-Bus error: {0}")]
    DbusError(String),

    #[error("Icon not found: {0}")]
    IconNotFound(String),
}

/// Clipboard and paste errors
#[derive(Error, Debug)]
pub enum ClipboardError {
    #[error("Clipboard access error: {0}")]
    AccessError(String),

    #[error("Failed to set clipboard: {0}")]
    SetError(String),

    #[error("Paste simulation failed: {0}")]
    PasteError(String),

    #[error("wtype not found - please install wtype for paste simulation")]
    WtypeNotFound,
}

pub type Result<T> = std::result::Result<T, AppError>;
