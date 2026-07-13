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
use crate::encoder::{extract_sps_pps, EncodeError, Encoder};

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
        let timescale = crate::encoder::MP4_TIMESCALE;
        let timing = crate::encoder::mp4_sample_timing(frames, config.fps);

        let mp4_config = Mp4Config {
            major_brand: "isom".parse().unwrap(),
            minor_version: 512,
            compatible_brands: vec![
                "isom".parse().unwrap(),
                "iso2".parse().unwrap(),
                "avc1".parse().unwrap(),
            ],
            timescale,
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
            timescale,
            language: "und".to_string(),
            media_conf: MediaConfig::AvcConfig(avc_config),
        };

        writer
            .add_track(&track_config)
            .map_err(|e| EncodeError::OutputFailed(format!("add_track: {e}")))?;

        // Write the capture-timestamp timeline at microsecond precision.
        for (i, frame) in frames.iter().enumerate() {
            if frame.pixel_format != crate::capture::PixelFormat::H264 {
                return Err(EncodeError::EncodeFailed(
                    "WindowsEncoder expects H.264 encoded frames".into(),
                ));
            }

            let (start_time, duration) = timing[i];

            let sample = mp4::Mp4Sample {
                start_time,
                duration,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::StoredFrame;
    use crate::capture::PixelFormat;
    use std::sync::Arc;
    use std::time::Instant;

    /// Build a synthetic AVCC H.264 frame containing SPS, PPS, and one IDR slice.
    fn make_avcc_frame(sps: &[u8], pps: &[u8], idr: &[u8]) -> StoredFrame {
        let mut data = Vec::new();
        // SPS in AVCC: [4B length][NAL data]
        data.extend_from_slice(&(sps.len() as u32).to_be_bytes());
        data.extend_from_slice(sps);
        // PPS in AVCC
        data.extend_from_slice(&(pps.len() as u32).to_be_bytes());
        data.extend_from_slice(pps);
        // IDR in AVCC
        data.extend_from_slice(&(idr.len() as u32).to_be_bytes());
        data.extend_from_slice(idr);

        StoredFrame {
            data: Arc::new(data),
            width: 1920,
            height: 1080,
            stride: 0,
            pixel_format: PixelFormat::H264,
            timestamp: Instant::now(),
            is_sync: true,
        }
    }

    /// Build a synthetic NV12 frame (simulates encoder fallback).
    fn make_nv12_frame() -> StoredFrame {
        let y_plane = vec![128u8; 1920 * 1080];
        let uv_plane = vec![128u8; (1920 / 2) * (1080 / 2) * 2];
        let mut data = y_plane;
        data.extend_from_slice(&uv_plane);

        StoredFrame {
            data: Arc::new(data),
            width: 1920,
            height: 1080,
            stride: 1920,
            pixel_format: PixelFormat::Nv12,
            timestamp: Instant::now(),
            is_sync: true,
        }
    }

    #[test]
    fn test_extract_sps_pps_from_avcc() {
        // Realistic SPS and PPS NAL data for 1080p Baseline
        let sps = vec![
            0x67, 0x64, 0x00, 0x1E, 0xAC, 0x1F, 0x47, 0x8B, 0x09, 0x80, 0x00, 0x00, 0x03, 0x00,
            0x80, 0x00, 0x00, 0x0F, 0x00, 0x3C, 0x08, 0x04, 0x38, 0x10, 0x00, 0x00, 0x00,
        ];
        let pps = vec![0x68, 0xEE, 0x3C, 0x80];
        let idr = vec![0x65, 0x88, 0x84, 0x00, 0x01, 0x23, 0x45];

        let frame = make_avcc_frame(&sps, &pps, &idr);
        let result = extract_sps_pps(&[frame]);
        assert!(
            result.is_ok(),
            "extract_sps_pps should succeed: {:?}",
            result.err()
        );
        let (found_sps, found_pps) = result.unwrap();
        assert_eq!(found_sps, sps, "SPS should match");
        assert_eq!(found_pps, pps, "PPS should match");
    }

    #[test]
    fn test_extract_sps_pps_skips_nv12() {
        let nv12_frame = make_nv12_frame();
        let result = extract_sps_pps(&[nv12_frame]);
        assert!(result.is_err(), "extract_sps_pps on NV12 should fail");
    }

    #[test]
    fn test_extract_sps_pps_mixed_frames() {
        let sps = vec![0x67, 0x64, 0x00, 0x1E, 0xAC];
        let pps = vec![0x68, 0xEE, 0x3C, 0x80];
        let idr = vec![0x65, 0x88, 0x84, 0x00];

        let h264_frame = make_avcc_frame(&sps, &pps, &idr);
        let nv12_frame = make_nv12_frame();

        // NV12 first, H.264 second — should still find SPS/PPS
        let result = extract_sps_pps(&[nv12_frame, h264_frame]);
        assert!(
            result.is_ok(),
            "should find SPS/PPS in H.264 frame after NV12"
        );
        let (found_sps, found_pps) = result.unwrap();
        assert_eq!(found_sps, sps);
        assert_eq!(found_pps, pps);
    }

    #[test]
    fn test_extract_sps_pps_prepended() {
        // Simulate what save_clip does: prepend cached SPS/PPS to first frame
        let sps = vec![0x67, 0x64, 0x00, 0x1E, 0xAC];
        let pps = vec![0x68, 0xEE, 0x3C, 0x80];

        // Cached SPS/PPS in AVCC format (with 4-byte length prefix)
        let mut cached_sps = Vec::new();
        cached_sps.extend_from_slice(&(sps.len() as u32).to_be_bytes());
        cached_sps.extend_from_slice(&sps);

        let mut cached_pps = Vec::new();
        cached_pps.extend_from_slice(&(pps.len() as u32).to_be_bytes());
        cached_pps.extend_from_slice(&pps);

        // A non-keyframe H.264 frame (no SPS/PPS inside, just a P-slice)
        let p_slice = vec![0x41, 0x9A, 0x22, 0x01];
        let mut frame_data = Vec::new();
        frame_data.extend_from_slice(&(p_slice.len() as u32).to_be_bytes());
        frame_data.extend_from_slice(&p_slice);

        let mut frame = StoredFrame {
            data: Arc::new(frame_data),
            width: 1920,
            height: 1080,
            stride: 0,
            pixel_format: PixelFormat::H264,
            timestamp: Instant::now(),
            is_sync: false,
        };

        // Prepend cached SPS+PPS (same logic as save_clip)
        let mut combined = cached_sps;
        combined.extend_from_slice(&cached_pps);
        combined.extend_from_slice(&frame.data);
        frame.data = Arc::new(combined);
        frame.is_sync = true;

        let result = extract_sps_pps(&[frame]);
        assert!(result.is_ok(), "should find prepended SPS/PPS");
        let (found_sps, found_pps) = result.unwrap();
        assert_eq!(found_sps, sps, "SPS should match");
        assert_eq!(found_pps, pps, "PPS should match");
    }
}
