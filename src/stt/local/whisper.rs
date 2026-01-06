use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::{debug, info};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::config::AppConfig;
use crate::error::SttError;
use crate::stt::{AudioData, SttProvider, SttResult, TranscriptionResult};

pub struct WhisperProvider {
    ctx: Arc<Mutex<WhisperContext>>,
    #[allow(dead_code)]
    model_name: String,
}

impl WhisperProvider {
    pub async fn new(config: &AppConfig) -> SttResult<Self> {
        let model_path = config.model_path();
        let model_name = config.providers.whisper_local.model.clone();

        info!("Loading whisper model from {:?}", model_path);

        if !model_path.exists() {
            return Err(SttError::ModelError(format!(
                "Model file not found: {:?}. Please download the model first.",
                model_path
            )));
        }

        let path = model_path.clone();
        let ctx = tokio::task::spawn_blocking(move || {
            WhisperContext::new_with_params(
                path.to_str().unwrap(),
                WhisperContextParameters::default(),
            )
        })
        .await
        .map_err(|e| SttError::ModelError(format!("Failed to load model: {}", e)))?
        .map_err(|e| SttError::ModelError(format!("Failed to load model: {}", e)))?;

        info!("Whisper model loaded successfully");

        Ok(Self {
            ctx: Arc::new(Mutex::new(ctx)),
            model_name,
        })
    }
}

#[async_trait]
impl SttProvider for WhisperProvider {
    fn name(&self) -> &'static str {
        "whisper-local"
    }

    fn is_local(&self) -> bool {
        true
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
        let samples = audio.samples.clone();
        let lang = language.map(|s| s.to_string());
        let ctx = self.ctx.clone();

        debug!(
            "Transcribing {} samples ({:.2}s of audio)",
            samples.len(),
            audio.duration().as_secs_f32()
        );

        let text = tokio::task::spawn_blocking(move || {
            let ctx = ctx.blocking_lock();
            let mut state = ctx.create_state().map_err(|e| {
                SttError::TranscriptionError(format!("Failed to create state: {}", e))
            })?;

            let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

            // Set language if specified
            if let Some(ref lang) = lang {
                if lang != "auto" {
                    params.set_language(Some(lang));
                }
            }

            // Keep original language (don't translate to English)
            params.set_translate(false);

            // Disable printing to stdout
            params.set_print_special(false);
            params.set_print_progress(false);
            params.set_print_realtime(false);
            params.set_print_timestamps(false);

            state.full(params, &samples).map_err(|e| {
                SttError::TranscriptionError(format!("Transcription failed: {}", e))
            })?;

            // Get number of segments (returns i32 directly)
            let num_segments = state.full_n_segments();

            let mut text = String::new();
            for i in 0..num_segments {
                // Use get_segment which returns Option<WhisperSegment>
                if let Some(segment) = state.get_segment(i) {
                    match segment.to_str_lossy() {
                        Ok(segment_text) => text.push_str(&segment_text),
                        Err(e) => debug!("Failed to get segment text {}: {}", i, e),
                    }
                }
            }

            Ok::<String, SttError>(text.trim().to_string())
        })
        .await
        .map_err(|e| SttError::TranscriptionError(format!("Task failed: {}", e)))??;

        let processing_time = start.elapsed();
        debug!("Transcription completed in {:?}", processing_time);

        Ok(TranscriptionResult::new(text)
            .with_language(language.unwrap_or("auto"))
            .with_processing_time(processing_time))
    }

    async fn health_check(&self) -> SttResult<()> {
        // Just check if we can acquire the lock
        let _ctx = self.ctx.lock().await;
        Ok(())
    }
}
