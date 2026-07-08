//! Codec configuration and encoder settings.

/// Supported video codecs for clip encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    /// H.264 Advanced Video Coding (widest compatibility)
    H264,
    /// H.265 High Efficiency Video Coding (better compression, newer)
    H265,
    /// AV1 (open standard, best compression, may require software encoding)
    Av1,
}

/// Full configuration for a single encoding session.
#[derive(Debug, Clone)]
pub struct EncoderConfig {
    /// Video codec to use
    pub codec: Codec,
    /// Target bitrate in kilobits/sec.
    pub bitrate_kbps: u32,
    /// Output frame rate
    pub fps: u32,
    /// Keyframe interval (0 = automatic)
    pub keyframe_interval: u32,
    /// Output video width in pixels
    pub target_width: u32,
    /// Output video height in pixels
    pub target_height: u32,
}

impl Default for EncoderConfig {
    fn default() -> Self {
        Self {
            codec: Codec::H264,
            bitrate_kbps: 8_000,
            fps: 60,
            keyframe_interval: 120,
            target_width: 1920,
            target_height: 1080,
        }
    }
}
