//! Windows hardware encoder using Media Foundation H.264 MFT.
//!
//! Frames are pre-encoded by the shadow buffer pipeline into H.264 AVCC packets.
//! The [`WindowsEncoder`] only muxes those packets into an MP4 container —
//! no actual encoding happens during clip save (~0.1 s).

pub mod mf_encoder;

use bytes::Bytes;
use mp4::{AvcConfig, MediaConfig, Mp4Config, Mp4Writer, TrackConfig, TrackType};
use std::path::Path;

use crate::buffer::StoredFrame;
use crate::encoder::codecs::EncoderConfig;
use crate::encoder::{EncodeError, Encoder};

pub struct WindowsEncoder;

impl WindowsEncoder {
    pub fn new() -> Self {
        Self
    }
}

impl Encoder for WindowsEncoder {
    fn encode_clip(
        &mut self,
        frames: &[StoredFrame],
        output_path: &Path,
        config: &EncoderConfig,
    ) -> Result<(), EncodeError> {
        if frames.is_empty() {
            return Err(EncodeError::EncodeFailed("No frames to mux".into()));
        }

        // Extract SPS / PPS from the first keyframe
        let (sps, pps) = extract_sps_pps(frames)?;

        // Build MP4 container
        let mp4_config = Mp4Config {
            major_brand: "isom".parse().unwrap(),
            minor_version: 512,
            compatible_brands: vec![
                "isom".parse().unwrap(),
                "iso2".parse().unwrap(),
                "avc1".parse().unwrap(),
            ],
            timescale: config.fps,
        };

        let file = std::fs::File::create(output_path)
            .map_err(|e| EncodeError::OutputFailed(format!("Create output: {e}")))?;

        let mut writer = Mp4Writer::write_start(file, &mp4_config)
            .map_err(|e| EncodeError::OutputFailed(format!("Mp4Writer start: {e}")))?;

        let avc_config = AvcConfig {
            width: config.target_width as u16,
            height: config.target_height as u16,
            seq_param_set: sps,
            pic_param_set: pps,
        };

        let track_config = TrackConfig {
            track_type: TrackType::Video,
            timescale: config.fps,
            language: "und".to_string(),
            media_conf: MediaConfig::AvcConfig(avc_config),
        };

        writer
            .add_track(&track_config)
            .map_err(|e| EncodeError::OutputFailed(format!("add_track: {e}")))?;

        // Write each frame's compressed data as an MP4 sample
        for (i, frame) in frames.iter().enumerate() {
            if frame.pixel_format != crate::capture::PixelFormat::H264 {
                return Err(EncodeError::EncodeFailed(
                    "WindowsEncoder expects H.264 encoded frames".into(),
                ));
            }

            let sample = mp4::Mp4Sample {
                start_time: i as u64,
                duration: 1,
                rendering_offset: 0,
                is_sync: frame.is_sync,
                bytes: Bytes::copy_from_slice(&frame.data),
            };

            writer
                .write_sample(1, &sample)
                .map_err(|e| EncodeError::EncodeFailed(format!("write_sample {i}: {e}")))?;
        }

        writer
            .write_end()
            .map_err(|e| EncodeError::OutputFailed(format!("write_end: {e}")))?;

        Ok(())
    }
}

/// Extract SPS and PPS NAL units from the frame list.
///
/// Scans every frame (not just keyframes) because SPS/PPS may have been
/// prepended to the first frame in the clip if the original keyframe was
/// evicted from the ring buffer. Data is expected in AVCC format (4-byte
/// big-endian length prefix per NAL).
fn extract_sps_pps(frames: &[StoredFrame]) -> Result<(Vec<u8>, Vec<u8>), EncodeError> {
    for frame in frames {
        let data = &frame.data;
        let mut offset = 0;
        let mut sps = Vec::new();
        let mut pps = Vec::new();

        while offset + 4 <= data.len() {
            let nal_len =
                u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;
            if offset + nal_len > data.len() {
                break;
            }
            let nal_type = data[offset] & 0x1F;
            match nal_type {
                7 => sps = data[offset..offset + nal_len].to_vec(),
                8 => pps = data[offset..offset + nal_len].to_vec(),
                _ => {}
            }
            offset += nal_len;
        }

        if !sps.is_empty() && !pps.is_empty() {
            return Ok((sps, pps));
        }
    }

    Err(EncodeError::EncodeFailed(
        "No SPS/PPS found in H.264 stream".into(),
    ))
}
