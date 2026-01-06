use cpal::Stream;
use std::sync::Arc;
use tokio::sync::{watch, Mutex};
use tracing::{debug, error, info, warn};

use crate::audio::{AudioCapture, CaptureConfig};
use crate::clipboard;
use crate::config::AppConfig;
use crate::error::{AppError, Result};
use crate::ipc::IpcCommand;
use crate::stt::{self, AudioData, SttProvider};
use crate::tray::TrayState;

/// Application states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    /// Ready to record
    Idle,
    /// Currently recording audio
    Recording,
    /// Processing/transcribing audio
    Processing,
    /// Error state
    Error,
}

/// Main application
pub struct App {
    config: AppConfig,
    state_tx: watch::Sender<AppState>,
    state_rx: watch::Receiver<AppState>,
    provider: Arc<Mutex<Option<Box<dyn SttProvider>>>>,
    audio_capture: Arc<Mutex<Option<AudioCapture>>>,
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    // Store stream separately - it's not Send so we use a std Mutex
    #[allow(dead_code)]
    audio_stream: Arc<std::sync::Mutex<Option<Stream>>>,
    // Store the audio collection task handle so we can await it
    audio_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl App {
    pub async fn new(config: AppConfig) -> Result<Self> {
        let (state_tx, state_rx) = watch::channel(AppState::Idle);

        Ok(Self {
            config,
            state_tx,
            state_rx,
            provider: Arc::new(Mutex::new(None)),
            audio_capture: Arc::new(Mutex::new(None)),
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
            audio_stream: Arc::new(std::sync::Mutex::new(None)),
            audio_task: Arc::new(Mutex::new(None)),
        })
    }

    /// Initialize the STT provider
    pub async fn init_provider(&self) -> Result<()> {
        info!(
            "Initializing STT provider: {}",
            self.config.general.default_provider
        );

        let provider = stt::create_provider(self.config.general.default_provider, &self.config)
            .await
            .map_err(|e| AppError::Stt(e))?;

        *self.provider.lock().await = Some(provider);
        info!("STT provider initialized");
        Ok(())
    }

    /// Get a receiver for state changes
    pub fn state_receiver(&self) -> watch::Receiver<AppState> {
        self.state_rx.clone()
    }

    /// Get the current state
    pub fn state(&self) -> AppState {
        *self.state_rx.borrow()
    }

    /// Handle an IPC command
    pub async fn handle_command(&self, cmd: IpcCommand) -> Result<()> {
        match cmd {
            IpcCommand::Toggle => {
                match self.state() {
                    AppState::Idle => self.start_recording().await?,
                    AppState::Recording => self.stop_and_transcribe().await?,
                    _ => {
                        debug!("Ignoring toggle command in {:?} state", self.state());
                    }
                }
            }
            IpcCommand::Start => {
                if self.state() == AppState::Idle {
                    self.start_recording().await?;
                }
            }
            IpcCommand::Stop => {
                if self.state() == AppState::Recording {
                    self.stop_and_transcribe().await?;
                }
            }
            IpcCommand::Cancel => {
                self.cancel().await?;
            }
            IpcCommand::Status => {
                info!("Current state: {:?}", self.state());
            }
            IpcCommand::Shutdown => {
                info!("Shutdown requested");
                return Err(AppError::Other("Shutdown".into()));
            }
        }
        Ok(())
    }

    /// Start recording audio
    async fn start_recording(&self) -> Result<()> {
        info!("Starting recording");

        // Forcibly abort any lingering audio task from a previous session
        if let Some(task) = self.audio_task.lock().await.take() {
            debug!("Aborting old audio task before starting new recording");
            task.abort();
        }

        // Clear audio buffer
        self.audio_buffer.lock().await.clear();

        // Create audio capture
        let capture_config = CaptureConfig {
            sample_rate: self.config.audio.sample_rate,
            device_name: self.config.audio.input_device.clone(),
            ..Default::default()
        };

        let capture = AudioCapture::new(capture_config).map_err(AppError::Audio)?;
        let (stream, mut rx) = capture.start().map_err(AppError::Audio)?;

        // Store capture and stream (keeps them alive)
        *self.audio_capture.lock().await = Some(capture);
        *self.audio_stream.lock().unwrap() = Some(stream);

        // Update state
        self.set_state(AppState::Recording);

        // Spawn task to collect audio samples (rx is Send, stream is not)
        let buffer = self.audio_buffer.clone();
        let mut state_rx = self.state_rx.clone();
        let max_duration = self.config.audio.max_duration;

        let task = tokio::spawn(async move {
            let start = std::time::Instant::now();
            let max_duration = std::time::Duration::from_secs(max_duration as u64);

            loop {
                // Check if we should stop
                if *state_rx.borrow() != AppState::Recording {
                    debug!("Audio collection task: state changed, exiting");
                    break;
                }

                // Check timeout
                if start.elapsed() > max_duration {
                    warn!("Max recording duration reached");
                    break;
                }

                tokio::select! {
                    biased;

                    // Watch for state changes
                    _ = state_rx.changed() => {
                        debug!("Audio collection task: state change detected");
                        continue;
                    }

                    // Receive audio with timeout
                    result = tokio::time::timeout(
                        std::time::Duration::from_millis(50),
                        rx.recv()
                    ) => {
                        match result {
                            Ok(Some(samples)) => {
                                buffer.lock().await.extend(samples);
                            }
                            Ok(None) => {
                                // Channel closed
                                debug!("Audio channel closed");
                                break;
                            }
                            Err(_) => {
                                // Timeout, continue loop to check state
                            }
                        }
                    }
                }
            }
            debug!("Audio collection task finished");
        });

        // Store the task handle so we can await it later
        *self.audio_task.lock().await = Some(task);

        Ok(())
    }

    /// Stop recording and transcribe
    async fn stop_and_transcribe(&self) -> Result<()> {
        info!("Stopping recording and transcribing");

        // Update state first to signal the audio collection task to stop
        self.set_state(AppState::Processing);

        // Stop audio capture (sets is_recording to false in callback)
        if let Some(capture) = self.audio_capture.lock().await.take() {
            capture.stop();
        }

        // Drop the stream to close the channel sender
        *self.audio_stream.lock().unwrap() = None;

        // Abort the audio collection task - we already have the audio in the buffer
        if let Some(task) = self.audio_task.lock().await.take() {
            debug!("Aborting audio collection task");
            task.abort();
        }

        // Get audio data
        let samples = self.audio_buffer.lock().await.clone();

        if samples.is_empty() {
            warn!("No audio recorded");
            self.set_state(AppState::Idle);
            return Ok(());
        }

        let audio = AudioData::new(samples, self.config.audio.sample_rate);
        info!(
            "Recorded {:.2}s of audio",
            audio.duration().as_secs_f32()
        );

        // Transcribe
        let provider = self.provider.lock().await;
        let provider = provider
            .as_ref()
            .ok_or_else(|| AppError::Other("Provider not initialized".into()))?;

        let language = if self.config.general.language == "auto" {
            None
        } else {
            Some(self.config.general.language.as_str())
        };

        match provider.transcribe(&audio, language).await {
            Ok(result) => {
                info!(
                    "Transcription: \"{}\" ({:?})",
                    result.text, result.processing_time
                );

                if !result.text.is_empty() {
                    // Paste if enabled
                    if self.config.general.auto_paste {
                        if let Err(e) = clipboard::paste_text(&result.text).await {
                            error!("Failed to paste: {}", e);
                            // Still copy to clipboard at least
                            let _ = clipboard::set_clipboard(&result.text).await;
                        }
                    } else {
                        // Just copy to clipboard
                        let _ = clipboard::set_clipboard(&result.text).await;
                    }
                }

                self.set_state(AppState::Idle);
            }
            Err(e) => {
                error!("Transcription failed: {}", e);
                self.set_state(AppState::Error);
                // Recover to idle after a moment
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                self.set_state(AppState::Idle);
            }
        }

        Ok(())
    }

    /// Cancel current operation
    async fn cancel(&self) -> Result<()> {
        info!("Cancelling operation");

        // Set state first to signal the audio collection task to stop
        self.set_state(AppState::Idle);

        // Stop audio capture and drop stream
        if let Some(capture) = self.audio_capture.lock().await.take() {
            capture.stop();
        }
        *self.audio_stream.lock().unwrap() = None;

        // Abort the audio collection task
        if let Some(task) = self.audio_task.lock().await.take() {
            debug!("Aborting audio collection task");
            task.abort();
        }

        // Clear buffer
        self.audio_buffer.lock().await.clear();

        Ok(())
    }

    fn set_state(&self, state: AppState) {
        debug!("State: {:?} -> {:?}", *self.state_tx.borrow(), state);
        let _ = self.state_tx.send(state);
    }
}

impl AppState {
    pub fn to_tray_state(self) -> TrayState {
        match self {
            AppState::Idle => TrayState::Idle,
            AppState::Recording => TrayState::Recording,
            AppState::Processing => TrayState::Processing,
            AppState::Error => TrayState::Error,
        }
    }
}
