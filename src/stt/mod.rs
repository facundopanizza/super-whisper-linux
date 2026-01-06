pub mod provider;
pub mod local;
pub mod cloud;

pub use provider::{AudioData, SttProvider, SttResult, TranscriptionResult};

use crate::config::{AppConfig, ProviderType};

/// Create a provider based on the configuration
pub async fn create_provider(
    provider_type: ProviderType,
    config: &AppConfig,
) -> SttResult<Box<dyn SttProvider>> {
    match provider_type {
        ProviderType::WhisperLocal => {
            let provider = local::WhisperProvider::new(config).await?;
            Ok(Box::new(provider))
        }
        ProviderType::OpenAI => {
            let provider = cloud::OpenAIProvider::new(config)?;
            Ok(Box::new(provider))
        }
        ProviderType::Groq => {
            let provider = cloud::GroqProvider::new(config)?;
            Ok(Box::new(provider))
        }
        ProviderType::Deepgram => {
            let provider = cloud::DeepgramProvider::new(config)?;
            Ok(Box::new(provider))
        }
    }
}
