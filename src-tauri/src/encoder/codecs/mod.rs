//! Codec configuration and encoder settings.

/// Supported video codecs for clip encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    /// H.264 Advanced Video Coding (widest compatibility)
    H264,
    /// H.265 High Efficiency Video Coding (better compression, newer)
    #[allow(dead_code)]
    H265,
    /// AV1 (open standard, best compression, may require software encoding)
    #[allow(dead_code)]
    Av1,
}

/// Full configuration for a single encoding session.
#[derive(Debug, Clone)]
pub struct EncoderConfig {
    /// Video codec to use
    #[allow(dead_code)]
    pub codec: Codec,
    /// Target bitrate in kilobits/sec.
    #[allow(dead_code)]
    pub bitrate_kbps: u32,
    /// Output frame rate
    #[allow(dead_code)]
    pub fps: u32,
    /// Keyframe interval (0 = automatic)
    #[allow(dead_code)]
    pub keyframe_interval: u32,
    /// Output video width in pixels
    #[allow(dead_code)]
    pub target_width: u32,
    /// Output video height in pixels
    #[allow(dead_code)]
    pub target_height: u32,
    /// Optional AAC audio track to mux alongside the video.
    #[allow(dead_code)]
    pub audio: Option<AudioClip>,
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
            audio: None,
        }
    }
}

/// A single raw AAC frame (1024 samples, no ADTS header) for MP4 muxing.
#[derive(Debug, Clone)]
pub struct AacFrame {
    /// Sample start time, in `sample_rate` units (e.g. 48 kHz timescale).
    pub start_time: u64,
    /// Samples per AAC frame (always 1024).
    pub duration: u32,
    /// Raw AAC payload bytes.
    pub data: Vec<u8>,
}

/// An encoded AAC audio clip attached to a clip save.
#[derive(Debug, Clone)]
pub struct AudioClip {
    pub sample_rate: u32,
    pub channels: u8,
    pub bitrate_kbps: u32,
    pub frames: Vec<AacFrame>,
}
