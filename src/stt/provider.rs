use async_trait::async_trait;
use std::time::Duration;

use crate::error::SttError;

pub type SttResult<T> = std::result::Result<T, SttError>;

/// Audio data for transcription
#[derive(Debug, Clone)]
pub struct AudioData {
    /// Raw audio samples (f32, mono, 16kHz)
    pub samples: Vec<f32>,
    /// Sample rate (should be 16000 for whisper)
    pub sample_rate: u32,
}

impl AudioData {
    pub fn new(samples: Vec<f32>, sample_rate: u32) -> Self {
        Self {
            samples,
            sample_rate,
        }
    }

    /// Duration of the audio in seconds
    pub fn duration(&self) -> Duration {
        Duration::from_secs_f32(self.samples.len() as f32 / self.sample_rate as f32)
    }

    /// Check if the audio is empty or too short
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty() || self.duration() < Duration::from_millis(100)
    }
}

/// Transcription result from any provider
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    /// The transcribed text
    pub text: String,
    /// Detected or specified language
    pub language: Option<String>,
    /// Confidence score (0.0 - 1.0) if available
    pub confidence: Option<f32>,
    /// Processing time
    pub processing_time: Duration,
}

impl TranscriptionResult {
    pub fn new(text: String) -> Self {
        Self {
            text,
            language: None,
            confidence: None,
            processing_time: Duration::ZERO,
        }
    }

    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = Some(confidence);
        self
    }

    pub fn with_processing_time(mut self, duration: Duration) -> Self {
        self.processing_time = duration;
        self
    }
}

/// Speech-to-text provider trait
#[async_trait]
pub trait SttProvider: Send + Sync {
    /// Returns the provider name for logging/display
    fn name(&self) -> &'static str;

    /// Returns true if this is a local (offline) provider
    fn is_local(&self) -> bool;

    /// Transcribe audio data to text
    async fn transcribe(&self, audio: &AudioData, language: Option<&str>) -> SttResult<TranscriptionResult>;

    /// Check if provider is ready (model loaded, API reachable)
    async fn health_check(&self) -> SttResult<()>;

    /// Get estimated cost per minute (for cloud providers)
    fn cost_per_minute(&self) -> Option<f64> {
        None
    }
}
