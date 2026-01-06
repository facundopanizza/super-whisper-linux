use hound::{SampleFormat, WavSpec, WavWriter};
use std::io::Cursor;

use crate::error::AudioError;

/// Encode audio samples as WAV data
pub fn encode_wav(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>, AudioError> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    let mut writer =
        WavWriter::new(&mut cursor, spec).map_err(|e| AudioError::WavError(e.to_string()))?;

    for &sample in samples {
        // Convert f32 [-1.0, 1.0] to i16
        let sample_i16 = (sample * i16::MAX as f32) as i16;
        writer
            .write_sample(sample_i16)
            .map_err(|e| AudioError::WavError(e.to_string()))?;
    }

    writer
        .finalize()
        .map_err(|e| AudioError::WavError(e.to_string()))?;

    Ok(cursor.into_inner())
}
