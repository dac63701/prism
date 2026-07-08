//! Windows hardware encoder stub — will use NVENC via the `windows` crate.
// TODO: Implement NVENC-based encoder
use crate::buffer::StoredFrame;
use crate::encoder::codecs::EncoderConfig;
use crate::encoder::{EncodeError, Encoder};
use std::path::Path;

pub struct WindowsEncoder;

impl WindowsEncoder {
    pub fn new() -> Self {
        Self
    }
}

impl Encoder for WindowsEncoder {
    fn encode_clip(
        &mut self,
        _frames: &[StoredFrame],
        _output_path: &Path,
        _config: &EncoderConfig,
    ) -> Result<(), EncodeError> {
        Err(EncodeError::UnsupportedPlatform)
    }
}
