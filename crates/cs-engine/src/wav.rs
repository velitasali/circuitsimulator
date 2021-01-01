//! WAV audio reader matching C++ `WaveGen::setFile`.

use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;

/// Decoded audio data from a WAV file.
#[derive(Clone, Debug, PartialEq)]
pub struct WavData {
    /// Audio samples normalized to `[0.0, 1.0]`.
    pub samples: Arc<Vec<f64>>,
    /// Sample rate in Hz (e.g. 44100).
    pub sample_rate: u32,
    /// Number of audio channels in the source file.
    pub num_channels: u16,
    /// Bits per sample.
    pub bits_per_sample: u16,
}

impl WavData {
    /// Decode a WAV file from the filesystem.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path_ref = path.as_ref();
        let mut file = File::open(path_ref)
            .map_err(|e| format!("Failed to open WAV file {}: {e}", path_ref.display()))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| format!("Failed to read WAV file {}: {e}", path_ref.display()))?;
        Self::from_bytes(&bytes)
    }

    /// Decode WAV data from an in-memory byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut r = Cursor::new(bytes);

        // Read RIFF header
        let mut riff = [0u8; 4];
        r.read_exact(&mut riff)
            .map_err(|_| "WAV file too short".to_string())?;
        if &riff != b"RIFF" {
            return Err("Invalid WAV file: missing RIFF header".to_string());
        }

        let mut u32_buf = [0u8; 4];
        r.read_exact(&mut u32_buf)
            .map_err(|_| "WAV file too short for file size".to_string())?;

        let mut wave = [0u8; 4];
        r.read_exact(&mut wave)
            .map_err(|_| "WAV file too short for WAVE header".to_string())?;
        if &wave != b"WAVE" {
            return Err("Invalid WAV file: missing WAVE header".to_string());
        }

        let mut audio_format = 0u16;
        let mut num_channels = 0u16;
        let mut sample_rate = 0u32;
        let mut block_size = 0u16;
        let mut bits_per_sample = 0u16;
        let mut data_bytes = Vec::new();

        let mut u16_buf = [0u8; 2];

        // Parse chunks until EOF
        while let Ok(()) = r.read_exact(&mut riff) {
            r.read_exact(&mut u32_buf)
                .map_err(|_| "Malformed WAV chunk header".to_string())?;
            let chunk_size = u32::from_le_bytes(u32_buf) as usize;

            if &riff == b"fmt " {
                if chunk_size < 16 {
                    return Err("Malformed fmt chunk: size < 16".to_string());
                }
                r.read_exact(&mut u16_buf).map_err(|_| "fmt read error")?;
                audio_format = u16::from_le_bytes(u16_buf);

                r.read_exact(&mut u16_buf).map_err(|_| "fmt read error")?;
                num_channels = u16::from_le_bytes(u16_buf);

                r.read_exact(&mut u32_buf).map_err(|_| "fmt read error")?;
                sample_rate = u32::from_le_bytes(u32_buf);

                // Skip byte rate (4 bytes)
                let mut dummy4 = [0u8; 4];
                r.read_exact(&mut dummy4).map_err(|_| "fmt read error")?;

                r.read_exact(&mut u16_buf).map_err(|_| "fmt read error")?;
                block_size = u16::from_le_bytes(u16_buf);

                r.read_exact(&mut u16_buf).map_err(|_| "fmt read error")?;
                bits_per_sample = u16::from_le_bytes(u16_buf);

                // Skip any remaining bytes in the fmt chunk (e.g. cbSize in extensible fmt)
                if chunk_size > 16 {
                    r.seek(SeekFrom::Current((chunk_size - 16) as i64))
                        .map_err(|_| "Failed to skip extra fmt bytes".to_string())?;
                }
            } else if &riff == b"data" {
                let start = r.position() as usize;
                let end = (start + chunk_size).min(bytes.len());
                if end > start {
                    data_bytes = bytes[start..end].to_vec();
                }
                break;
            } else {
                // Skip unknown chunk
                r.seek(SeekFrom::Current(chunk_size as i64))
                    .map_err(|_| "Failed to skip unknown chunk".to_string())?;
            }
        }

        if num_channels == 0 {
            return Err("Invalid WAV: 0 channels".to_string());
        }
        if sample_rate == 0 {
            return Err("Invalid WAV: 0 sample rate".to_string());
        }

        let bytes_per_sample = if block_size > 0 && num_channels > 0 {
            (block_size / num_channels) as usize
        } else {
            (bits_per_sample / 8) as usize
        };

        if bytes_per_sample == 0 {
            return Err("Invalid WAV: 0 bytes per sample".to_string());
        }

        let frame_size = bytes_per_sample * num_channels as usize;
        if frame_size == 0 {
            return Err("Invalid WAV: 0 frame size".to_string());
        }

        let total_frames = data_bytes.len() / frame_size;
        let mut samples = Vec::with_capacity(total_frames);

        match audio_format {
            1 => {
                // PCM
                if bytes_per_sample == 1 {
                    // 8-bit unsigned: 0..255 -> 0.0..1.0
                    for frame in 0..total_frames {
                        let offset = frame * frame_size; // Channel 0 is first byte
                        let val = data_bytes[offset] as f64;
                        samples.push((val / 255.0).clamp(0.0, 1.0));
                    }
                } else if bytes_per_sample == 2 {
                    // 16-bit signed LE: -32768..32767 -> 0.0..1.0
                    for frame in 0..total_frames {
                        let offset = frame * frame_size;
                        let val =
                            i16::from_le_bytes([data_bytes[offset], data_bytes[offset + 1]]) as f64;
                        let normalized = (val - (-32768.0)) / (32767.0 - (-32768.0));
                        samples.push(normalized.clamp(0.0, 1.0));
                    }
                } else if bytes_per_sample == 3 {
                    // 24-bit signed LE
                    for frame in 0..total_frames {
                        let offset = frame * frame_size;
                        let b0 = data_bytes[offset] as i32;
                        let b1 = data_bytes[offset + 1] as i32;
                        let b2 = data_bytes[offset + 2] as i8 as i32; // sign-extended
                        let val = (b2 << 16) | (b1 << 8) | b0;
                        let normalized = (val as f64 + 8388608.0) / 16777215.0;
                        samples.push(normalized.clamp(0.0, 1.0));
                    }
                } else {
                    return Err(format!(
                        "Unsupported PCM bytes per sample: {bytes_per_sample}"
                    ));
                }
            }
            3 => {
                // IEEE_FLOAT
                if bytes_per_sample == 4 {
                    // 32-bit float LE: -1.0..1.0 -> 0.0..1.0
                    for frame in 0..total_frames {
                        let offset = frame * frame_size;
                        let b = [
                            data_bytes[offset],
                            data_bytes[offset + 1],
                            data_bytes[offset + 2],
                            data_bytes[offset + 3],
                        ];
                        let val = f32::from_le_bytes(b) as f64;
                        let normalized = (val + 1.0) / 2.0;
                        samples.push(normalized.clamp(0.0, 1.0));
                    }
                } else if bytes_per_sample == 8 {
                    // 64-bit float LE: -1.0..1.0 -> 0.0..1.0
                    for frame in 0..total_frames {
                        let offset = frame * frame_size;
                        let b = [
                            data_bytes[offset],
                            data_bytes[offset + 1],
                            data_bytes[offset + 2],
                            data_bytes[offset + 3],
                            data_bytes[offset + 4],
                            data_bytes[offset + 5],
                            data_bytes[offset + 6],
                            data_bytes[offset + 7],
                        ];
                        let val = f64::from_le_bytes(b);
                        let normalized = (val + 1.0) / 2.0;
                        samples.push(normalized.clamp(0.0, 1.0));
                    }
                } else {
                    return Err(format!(
                        "Unsupported IEEE_FLOAT bytes per sample: {bytes_per_sample}"
                    ));
                }
            }
            _ => {
                return Err(format!("Unsupported WAV audio format: {audio_format}"));
            }
        }

        Ok(Self {
            samples: Arc::new(samples),
            sample_rate,
            num_channels,
            bits_per_sample,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_wav_16bit_pcm(samples: &[i16], sample_rate: u32, channels: u16) -> Vec<u8> {
        let mut buf = Vec::new();
        let bytes_per_sample = 2u16;
        let block_size = bytes_per_sample * channels;
        let byte_rate = sample_rate * block_size as u32;
        let data_size = (samples.len() * 2) as u32;
        let riff_size = 36 + data_size;

        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&riff_size.to_le_bytes());
        buf.extend_from_slice(b"WAVE");

        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes()); // Chunk size
        buf.extend_from_slice(&1u16.to_le_bytes()); // Audio format (1 = PCM)
        buf.extend_from_slice(&channels.to_le_bytes());
        buf.extend_from_slice(&sample_rate.to_le_bytes());
        buf.extend_from_slice(&byte_rate.to_le_bytes());
        buf.extend_from_slice(&block_size.to_le_bytes());
        buf.extend_from_slice(&16u16.to_le_bytes()); // Bits per sample

        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_size.to_le_bytes());
        for s in samples {
            buf.extend_from_slice(&s.to_le_bytes());
        }

        buf
    }

    #[test]
    fn test_parse_16bit_pcm_mono() {
        let raw = [-32768i16, 0, 32767];
        let wav_bytes = create_test_wav_16bit_pcm(&raw, 44100, 1);
        let wav = WavData::from_bytes(&wav_bytes).expect("Should decode mono 16-bit WAV");

        assert_eq!(wav.sample_rate, 44100);
        assert_eq!(wav.num_channels, 1);
        assert_eq!(wav.samples.len(), 3);
        assert!((wav.samples[0] - 0.0).abs() < 1e-4);
        assert!((wav.samples[1] - 0.5).abs() < 1e-3);
        assert!((wav.samples[2] - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_parse_16bit_pcm_stereo() {
        // Interleaved stereo samples: L0, R0, L1, R1
        let raw = [-32768i16, 32767, 32767, -32768];
        let wav_bytes = create_test_wav_16bit_pcm(&raw, 22050, 2);
        let wav = WavData::from_bytes(&wav_bytes).expect("Should decode stereo 16-bit WAV");

        assert_eq!(wav.sample_rate, 22050);
        assert_eq!(wav.num_channels, 2);
        assert_eq!(wav.samples.len(), 2);
        assert!((wav.samples[0] - 0.0).abs() < 1e-4);
        assert!((wav.samples[1] - 1.0).abs() < 1e-4);
    }
}
