use async_trait::async_trait;
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde::Deserialize;
use std::time::Instant;
use tracing::debug;

use crate::config::AppConfig;
use crate::error::SttError;
use crate::stt::{AudioData, SttProvider, SttResult, TranscriptionResult};

#[derive(Debug, Deserialize)]
struct GroqResponse {
    text: String,
}

pub struct GroqProvider {
    client: Client,
    api_key: String,
    model: String,
    endpoint: String,
}

impl GroqProvider {
    pub fn new(config: &AppConfig) -> SttResult<Self> {
        let api_key = config
            .groq_api_key()
            .ok_or_else(|| SttError::ProviderUnavailable("Groq API key not configured".into()))?;

        let client = Client::new();

        Ok(Self {
            client,
            api_key,
            model: config.providers.groq.model.clone(),
            endpoint: config.providers.groq.endpoint.clone(),
        })
    }
}

#[async_trait]
impl SttProvider for GroqProvider {
    fn name(&self) -> &'static str {
        "groq"
    }

    fn is_local(&self) -> bool {
        false
    }

    fn cost_per_minute(&self) -> Option<f64> {
        Some(0.0) // Groq has a free tier
    }

    async fn transcribe(
        &self,
        audio: &AudioData,
        language: Option<&str>,
    ) -> SttResult<TranscriptionResult> {
        if audio.is_empty() {
            return Err(SttError::InvalidAudio("Audio is empty or too short".into()));
        }

        let start = Instant::now();

        // Encode audio as WAV
        let wav_data = crate::audio::encode_wav(&audio.samples, audio.sample_rate)
            .map_err(|e| SttError::InvalidAudio(e.to_string()))?;

        debug!("Sending {} bytes to Groq API", wav_data.len());

        let file_part = Part::bytes(wav_data)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| SttError::ApiError(format!("Failed to create multipart: {}", e)))?;

        let mut form = Form::new()
            .part("file", file_part)
            .text("model", self.model.clone());

        if let Some(lang) = language {
            if lang != "auto" {
                form = form.text("language", lang.to_string());
            }
        }

        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SttError::ApiError(format!(
                "Groq API error {}: {}",
                status, body
            )));
        }

        let result: GroqResponse = response.json().await?;
        let processing_time = start.elapsed();

        debug!("Groq transcription completed in {:?}", processing_time);

        Ok(TranscriptionResult::new(result.text)
            .with_language(language.unwrap_or("auto"))
            .with_processing_time(processing_time))
    }

    async fn health_check(&self) -> SttResult<()> {
        if self.api_key.is_empty() {
            return Err(SttError::ProviderUnavailable("API key is empty".into()));
        }
        Ok(())
    }
}
