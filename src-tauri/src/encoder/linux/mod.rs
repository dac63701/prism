//! Linux hardware encoder stub — will use VAAPI via `ffmpeg` or `gstreamer`.
// TODO: Implement VAAPI-based encoder
use crate::buffer::StoredFrame;
use crate::encoder::codecs::EncoderConfig;
use crate::encoder::{EncodeError, Encoder};
use std::path::Path;

pub struct LinuxEncoder;

impl LinuxEncoder {
    pub fn new() -> Self {
        Self
    }
}

impl Encoder for LinuxEncoder {
    fn encode_clip(
        &mut self,
        _frames: &[StoredFrame],
        _output_path: &Path,
        _config: &EncoderConfig,
    ) -> Result<(), EncodeError> {
        Err(EncodeError::UnsupportedPlatform)
    }
}
