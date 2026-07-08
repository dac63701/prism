//! Video encoding and transcoding module.
//!
//! Provides a platform-agnostic [`Encoder`] trait and a factory function
//! that returns the platform-native hardware encoder.

pub mod codecs;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "linux")]
pub mod linux;

use std::path::Path;
use codecs::EncoderConfig;
use crate::buffer::StoredFrame;

/// Errors that can occur during encoding.
#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    #[error("Encoder not available on this platform")]
    UnsupportedPlatform,

    #[error("Failed to initialize encoder: {0}")]
    InitFailed(String),

    #[error("Encoding frame failed: {0}")]
    EncodeFailed(String),

    #[error("Failed to write output file: {0}")]
    OutputFailed(String),

    #[error("Unsupported codec: {0}")]
    UnsupportedCodec(String),
}

/// Platform-agnostic video encoder.
///
/// Implementations take raw frames from the ring buffer, encode them using
/// platform-native hardware encoders, and mux the result into an MP4 file.
pub trait Encoder: Send {
    /// Encode a slice of frames and write the result to `output_path`.
    fn encode_clip(
        &mut self,
        frames: &[StoredFrame],
        output_path: &Path,
        config: &EncoderConfig,
    ) -> Result<(), EncodeError>;
}

/// Create the platform-appropriate hardware encoder.
pub fn create_encoder() -> Box<dyn Encoder> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacEncoder::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsEncoder::new())
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxEncoder::new())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Box::new(UnsupportedEncoder)
    }
}

// ---------------------------------------------------------------------------
// Fallback encoder for unsupported platforms
// ---------------------------------------------------------------------------

pub struct UnsupportedEncoder;

impl Encoder for UnsupportedEncoder {
    fn encode_clip(
        &mut self,
        _frames: &[StoredFrame],
        _output_path: &Path,
        _config: &EncoderConfig,
    ) -> Result<(), EncodeError> {
        Err(EncodeError::UnsupportedPlatform)
    }
}
