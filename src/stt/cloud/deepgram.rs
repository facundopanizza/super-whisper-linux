use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::time::Instant;
use tracing::debug;

use crate::config::AppConfig;
use crate::error::SttError;
use crate::stt::{AudioData, SttProvider, SttResult, TranscriptionResult};

#[derive(Debug, Deserialize)]
struct DeepgramResponse {
    results: DeepgramResults,
}

#[derive(Debug, Deserialize)]
struct DeepgramResults {
    channels: Vec<DeepgramChannel>,
}

#[derive(Debug, Deserialize)]
struct DeepgramChannel {
    alternatives: Vec<DeepgramAlternative>,
}

#[derive(Debug, Deserialize)]
struct DeepgramAlternative {
    transcript: String,
    confidence: f32,
}

pub struct DeepgramProvider {
    client: Client,
    api_key: String,
    model: String,
    features: Vec<String>,
}

impl DeepgramProvider {
    pub fn new(config: &AppConfig) -> SttResult<Self> {
        let api_key = config.deepgram_api_key().ok_or_else(|| {
            SttError::ProviderUnavailable("Deepgram API key not configured".into())
        })?;

        let client = Client::new();

        Ok(Self {
            client,
            api_key,
            model: config.providers.deepgram.model.clone(),
            features: config.providers.deepgram.features.clone(),
        })
    }

    fn build_url(&self, language: Option<&str>) -> String {
        let mut url = format!(
            "https://api.deepgram.com/v1/listen?model={}",
            self.model
        );

        for feature in &self.features {
            url.push_str(&format!("&{}=true", feature));
        }

        if let Some(lang) = language {
            if lang != "auto" {
                url.push_str(&format!("&language={}", lang));
            }
        }

        url
    }
}

#[async_trait]
impl SttProvider for DeepgramProvider {
    fn name(&self) -> &'static str {
        "deepgram"
    }

    fn is_local(&self) -> bool {
        false
    }

    fn cost_per_minute(&self) -> Option<f64> {
        Some(0.0043) // Nova-2 pricing
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

        debug!("Sending {} bytes to Deepgram API", wav_data.len());

        let url = self.build_url(language);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Token {}", self.api_key))
            .header("Content-Type", "audio/wav")
            .body(wav_data)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SttError::ApiError(format!(
                "Deepgram API error {}: {}",
                status, body
            )));
        }

        let result: DeepgramResponse = response.json().await?;
        let processing_time = start.elapsed();

        // Extract transcript from response
        let (text, confidence) = result
            .results
            .channels
            .first()
            .and_then(|c| c.alternatives.first())
            .map(|a| (a.transcript.clone(), a.confidence))
            .unwrap_or_default();

        debug!("Deepgram transcription completed in {:?}", processing_time);

        Ok(TranscriptionResult::new(text)
            .with_language(language.unwrap_or("auto"))
            .with_confidence(confidence)
            .with_processing_time(processing_time))
    }

    async fn health_check(&self) -> SttResult<()> {
        if self.api_key.is_empty() {
            return Err(SttError::ProviderUnavailable("API key is empty".into()));
        }
        Ok(())
    }
}
