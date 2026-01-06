use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::error::AudioError;

/// Configuration for audio capture
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    /// Target sample rate (default: 16000 for whisper)
    pub sample_rate: u32,
    /// Input device name (None = default)
    pub device_name: Option<String>,
    /// Channel buffer size
    pub buffer_size: usize,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            device_name: None,
            buffer_size: 4096,
        }
    }
}

/// Audio capture manager
pub struct AudioCapture {
    device: Device,
    stream_config: StreamConfig,
    target_sample_rate: u32,
    is_recording: Arc<AtomicBool>,
}

impl AudioCapture {
    /// Create a new audio capture instance
    pub fn new(config: CaptureConfig) -> Result<Self, AudioError> {
        let host = cpal::default_host();

        // Find the input device
        let device = if let Some(ref name) = config.device_name {
            host.input_devices()
                .map_err(|e| AudioError::DeviceError(e.to_string()))?
                .find(|d| d.name().map(|n| n == *name).unwrap_or(false))
                .ok_or_else(|| AudioError::DeviceError(format!("Device '{}' not found", name)))?
        } else {
            host.default_input_device()
                .ok_or(AudioError::NoInputDevice)?
        };

        info!("Using audio input device: {:?}", device.name());

        // Get supported config
        let supported_config = device
            .default_input_config()
            .map_err(|e| AudioError::ConfigError(e.to_string()))?;

        debug!("Supported config: {:?}", supported_config);

        let stream_config = StreamConfig {
            channels: 1, // Mono for whisper
            sample_rate: supported_config.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        Ok(Self {
            device,
            stream_config,
            target_sample_rate: config.sample_rate,
            is_recording: Arc::new(AtomicBool::new(false)),
        })
    }

    /// List available input devices
    pub fn list_devices() -> Result<Vec<String>, AudioError> {
        let host = cpal::default_host();
        let devices: Vec<String> = host
            .input_devices()
            .map_err(|e| AudioError::DeviceError(e.to_string()))?
            .filter_map(|d| d.name().ok())
            .collect();
        Ok(devices)
    }

    /// Start recording and return a receiver for audio samples
    pub fn start(&self) -> Result<(Stream, mpsc::Receiver<Vec<f32>>), AudioError> {
        let (tx, rx) = mpsc::channel::<Vec<f32>>(32);
        let is_recording = self.is_recording.clone();
        is_recording.store(true, Ordering::SeqCst);

        let source_sample_rate = self.stream_config.sample_rate.0;
        let target_sample_rate = self.target_sample_rate;

        let err_fn = |err| error!("Audio stream error: {}", err);

        let stream = match self.device.default_input_config().unwrap().sample_format() {
            SampleFormat::F32 => self.build_stream::<f32>(
                tx,
                is_recording,
                source_sample_rate,
                target_sample_rate,
                err_fn,
            )?,
            SampleFormat::I16 => self.build_stream::<i16>(
                tx,
                is_recording,
                source_sample_rate,
                target_sample_rate,
                err_fn,
            )?,
            SampleFormat::U16 => self.build_stream::<u16>(
                tx,
                is_recording,
                source_sample_rate,
                target_sample_rate,
                err_fn,
            )?,
            _ => return Err(AudioError::ConfigError("Unsupported sample format".into())),
        };

        stream
            .play()
            .map_err(|e| AudioError::StreamError(e.to_string()))?;

        info!("Audio recording started");
        Ok((stream, rx))
    }

    fn build_stream<T>(
        &self,
        tx: mpsc::Sender<Vec<f32>>,
        is_recording: Arc<AtomicBool>,
        source_rate: u32,
        target_rate: u32,
        err_fn: impl Fn(cpal::StreamError) + Send + 'static,
    ) -> Result<Stream, AudioError>
    where
        T: cpal::Sample + cpal::SizedSample + Send + 'static,
        f32: cpal::FromSample<T>,
    {
        let stream = self
            .device
            .build_input_stream(
                &self.stream_config,
                move |data: &[T], _: &cpal::InputCallbackInfo| {
                    if !is_recording.load(Ordering::SeqCst) {
                        return;
                    }

                    // Convert to f32
                    let samples: Vec<f32> = data
                        .iter()
                        .map(|s| cpal::Sample::from_sample(*s))
                        .collect();

                    // Resample if needed
                    let samples = if source_rate != target_rate {
                        resample(&samples, source_rate, target_rate)
                    } else {
                        samples
                    };

                    // Send samples (non-blocking)
                    let _ = tx.try_send(samples);
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioError::StreamError(e.to_string()))?;

        Ok(stream)
    }

    /// Stop recording
    pub fn stop(&self) {
        self.is_recording.store(false, Ordering::SeqCst);
        info!("Audio recording stopped");
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }
}

/// Simple linear interpolation resampling
/// For better quality, use rubato crate
fn resample(samples: &[f32], source_rate: u32, target_rate: u32) -> Vec<f32> {
    if source_rate == target_rate {
        return samples.to_vec();
    }

    let ratio = target_rate as f64 / source_rate as f64;
    let output_len = (samples.len() as f64 * ratio) as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_idx = i as f64 / ratio;
        let idx = src_idx as usize;
        let frac = src_idx - idx as f64;

        let sample = if idx + 1 < samples.len() {
            samples[idx] * (1.0 - frac as f32) + samples[idx + 1] * frac as f32
        } else if idx < samples.len() {
            samples[idx]
        } else {
            0.0
        };

        output.push(sample);
    }

    output
}
