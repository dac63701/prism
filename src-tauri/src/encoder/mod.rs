//! Video encoding and transcoding module.
//!
//! Provides a platform-agnostic [`Encoder`] trait and a factory function
//! that returns the platform-native hardware encoder.

pub mod codecs;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

use crate::buffer::StoredFrame;
use codecs::EncoderConfig;
use std::path::Path;

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

/// Compute the effective MP4 timescale from actual frame timestamps.
///
/// Uses the wall-clock timestamps on the first and last frame to derive the
/// true capture rate, rather than relying on `config.fps` (which can diverge
/// from the actual capture rate due to display refresh limits, FPS-limiters,
/// or encoder back-pressure).
///
/// Falls back to `config_fps` when fewer than 2 frames are available or the
/// timestamps are degenerate.
pub fn compute_timescale(frames: &[StoredFrame], config_fps: u32) -> u32 {
    if frames.len() < 2 {
        return config_fps.max(1);
    }
    let first_ts = frames[0].timestamp;
    let last_ts = frames[frames.len() - 1].timestamp;
    let actual_duration_ns = last_ts.duration_since(first_ts).as_nanos() as f64;
    if actual_duration_ns <= 0.0 {
        return config_fps.max(1);
    }
    let actual_fps = (frames.len() as f64) / (actual_duration_ns / 1_000_000_000.0);
    (actual_fps.round() as u32).max(1)
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
// Synthetic SPS/PPS generator (fallback when encoder doesn't provide them)
// ---------------------------------------------------------------------------

/// Generate valid H.264 Baseline SPS and PPS NAL units for the given
/// resolution. Used as a final fallback when the hardware encoder doesn't
/// include SPS/PPS in its output bitstream or media type.
///
/// Returns AVCC-formatted NAL data (4-byte big-endian length prefix).
pub fn generate_sps_pps(width: u32, height: u32, _fps: u32) -> (Vec<u8>, Vec<u8>) {
    let sps = build_sps_nal(width, height);
    let pps = build_pps_nal();

    // Wrap in AVCC format (4-byte length prefix)
    let mut sps_avcc = Vec::with_capacity(4 + sps.len());
    sps_avcc.extend_from_slice(&(sps.len() as u32).to_be_bytes());
    sps_avcc.extend_from_slice(&sps);

    let mut pps_avcc = Vec::with_capacity(4 + pps.len());
    pps_avcc.extend_from_slice(&(pps.len() as u32).to_be_bytes());
    pps_avcc.extend_from_slice(&pps);

    (sps_avcc, pps_avcc)
}

/// Build a complete H.264 Baseline SPS NAL unit (without AVCC wrapper).
fn build_sps_nal(width: u32, height: u32) -> Vec<u8> {
    let pic_width_in_mbs = width.div_ceil(16);
    let pic_height_in_map_units = height.div_ceil(16);
    let pic_width_in_mbs_minus1 = pic_width_in_mbs.saturating_sub(1);
    let pic_height_in_map_units_minus1 = pic_height_in_map_units.saturating_sub(1);

    // Frame cropping for non-16-aligned dimensions
    let crop_right = ((pic_width_in_mbs * 16).saturating_sub(width)) / 2;
    let crop_bottom = ((pic_height_in_map_units * 16).saturating_sub(height)) / 2;
    let frame_cropping_flag = if crop_right > 0 || crop_bottom > 0 {
        1u8
    } else {
        0u8
    };

    // ---- Bit writer ----
    let mut buf: Vec<u8> = Vec::new();
    let mut byte: u8 = 0;
    let mut bit: u8 = 0;

    fn w(b: &mut Vec<u8>, by: &mut u8, bi: &mut u8, val: u8) {
        *by = (*by << 1) | (val & 1);
        *bi += 1;
        if *bi == 8 {
            b.push(*by);
            *by = 0;
            *bi = 0;
        }
    }

    fn ue(b: &mut Vec<u8>, by: &mut u8, bi: &mut u8, val: u32) {
        let code_num = val;
        let mut tmp = code_num + 1;
        let mut leading = 0u32;
        let mut suffix = Vec::new();
        while tmp > 1 {
            suffix.push((tmp & 1) as u8);
            tmp >>= 1;
            leading += 1;
        }
        for _ in 0..leading {
            w(b, by, bi, 0);
        }
        w(b, by, bi, 1);
        for &v in suffix.iter().rev() {
            w(b, by, bi, v);
        }
    }

    fn u1(b: &mut Vec<u8>, by: &mut u8, bi: &mut u8, val: u8) {
        w(b, by, bi, val);
    }

    fn trailing(b: &mut Vec<u8>, by: &mut u8, bi: &mut u8) {
        u1(b, by, bi, 1);
        while *bi != 0 {
            u1(b, by, bi, 0);
        }
    }

    // NAL unit header: forbidden=0, nal_ref_idc=3, nal_unit_type=7 (SPS)
    w(&mut buf, &mut byte, &mut bit, 0);
    w(&mut buf, &mut byte, &mut bit, 1);
    w(&mut buf, &mut byte, &mut bit, 1);
    w(&mut buf, &mut byte, &mut bit, 0);
    w(&mut buf, &mut byte, &mut bit, 0);
    w(&mut buf, &mut byte, &mut bit, 1);
    w(&mut buf, &mut byte, &mut bit, 1);
    w(&mut buf, &mut byte, &mut bit, 1);
    // Byte complete: 0b01100111 = 0x67

    // profile_idc = 66 (Baseline)
    u1(&mut buf, &mut byte, &mut bit, 0);
    u1(&mut buf, &mut byte, &mut bit, 1);
    u1(&mut buf, &mut byte, &mut bit, 0);
    u1(&mut buf, &mut byte, &mut bit, 0);
    u1(&mut buf, &mut byte, &mut bit, 0);
    u1(&mut buf, &mut byte, &mut bit, 0);
    u1(&mut buf, &mut byte, &mut bit, 1);
    u1(&mut buf, &mut byte, &mut bit, 0);
    // Byte complete: 0b01000010 = 0x42

    // constraints: all 0
    for _ in 0..8 {
        u1(&mut buf, &mut byte, &mut bit, 0);
    }

    // level_idc = 40 (Level 4.0)
    u1(&mut buf, &mut byte, &mut bit, 0);
    u1(&mut buf, &mut byte, &mut bit, 0);
    u1(&mut buf, &mut byte, &mut bit, 1);
    u1(&mut buf, &mut byte, &mut bit, 0);
    u1(&mut buf, &mut byte, &mut bit, 1);
    u1(&mut buf, &mut byte, &mut bit, 0);
    u1(&mut buf, &mut byte, &mut bit, 0);
    u1(&mut buf, &mut byte, &mut bit, 0);
    // Byte complete: 0b00101000 = 0x28

    // RBSP starts here
    // seq_parameter_set_id = 0
    ue(&mut buf, &mut byte, &mut bit, 0);
    // log2_max_frame_num_minus4 = 0
    ue(&mut buf, &mut byte, &mut bit, 0);
    // pic_order_cnt_type = 0
    ue(&mut buf, &mut byte, &mut bit, 0);
    // log2_max_pic_order_cnt_lsb_minus4 = 0
    ue(&mut buf, &mut byte, &mut bit, 0);
    // max_num_ref_frames = 1
    ue(&mut buf, &mut byte, &mut bit, 1);
    // gaps_in_frame_num_value_allowed_flag = 0
    u1(&mut buf, &mut byte, &mut bit, 0);
    // pic_width_in_mbs_minus1
    ue(&mut buf, &mut byte, &mut bit, pic_width_in_mbs_minus1);
    // pic_height_in_map_units_minus1
    ue(
        &mut buf,
        &mut byte,
        &mut bit,
        pic_height_in_map_units_minus1,
    );
    // frame_mbs_only_flag = 1 (progressive)
    u1(&mut buf, &mut byte, &mut bit, 1);
    // direct_8x8_inference_flag = 1
    u1(&mut buf, &mut byte, &mut bit, 1);
    // frame_cropping_flag
    u1(&mut buf, &mut byte, &mut bit, frame_cropping_flag);
    if frame_cropping_flag == 1 {
        ue(&mut buf, &mut byte, &mut bit, 0); // frame_crop_left_offset
        ue(&mut buf, &mut byte, &mut bit, crop_right);
        ue(&mut buf, &mut byte, &mut bit, 0); // frame_crop_top_offset
        ue(&mut buf, &mut byte, &mut bit, crop_bottom);
    }
    // vui_parameters_present_flag = 0
    u1(&mut buf, &mut byte, &mut bit, 0);
    // rbsp_trailing_bits
    trailing(&mut buf, &mut byte, &mut bit);

    buf
}

/// Build a complete H.264 Baseline PPS NAL unit (without AVCC wrapper).
fn build_pps_nal() -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let mut byte: u8 = 0;
    let mut bit: u8 = 0;

    fn w(b: &mut Vec<u8>, by: &mut u8, bi: &mut u8, val: u8) {
        *by = (*by << 1) | (val & 1);
        *bi += 1;
        if *bi == 8 {
            b.push(*by);
            *by = 0;
            *bi = 0;
        }
    }

    fn ue(b: &mut Vec<u8>, by: &mut u8, bi: &mut u8, val: u32) {
        let code_num = val;
        let mut tmp = code_num + 1;
        let mut leading = 0u32;
        let mut suffix = Vec::new();
        while tmp > 1 {
            suffix.push((tmp & 1) as u8);
            tmp >>= 1;
            leading += 1;
        }
        for _ in 0..leading {
            w(b, by, bi, 0);
        }
        w(b, by, bi, 1);
        for &v in suffix.iter().rev() {
            w(b, by, bi, v);
        }
    }

    fn u1(b: &mut Vec<u8>, by: &mut u8, bi: &mut u8, val: u8) {
        w(b, by, bi, val);
    }

    fn trailing(b: &mut Vec<u8>, by: &mut u8, bi: &mut u8) {
        u1(b, by, bi, 1);
        while *bi != 0 {
            u1(b, by, bi, 0);
        }
    }

    // NAL unit header: forbidden=0, nal_ref_idc=3, nal_unit_type=8 (PPS)
    w(&mut buf, &mut byte, &mut bit, 0);
    w(&mut buf, &mut byte, &mut bit, 1);
    w(&mut buf, &mut byte, &mut bit, 1);
    w(&mut buf, &mut byte, &mut bit, 0);
    w(&mut buf, &mut byte, &mut bit, 1);
    w(&mut buf, &mut byte, &mut bit, 0);
    w(&mut buf, &mut byte, &mut bit, 0);
    w(&mut buf, &mut byte, &mut bit, 0);
    // Byte complete: 0b01101000 = 0x68

    // pic_parameter_set_id = 0
    ue(&mut buf, &mut byte, &mut bit, 0);
    // seq_parameter_set_id = 0
    ue(&mut buf, &mut byte, &mut bit, 0);
    // entropy_coding_mode_flag = 0 (CAVLC, Baseline)
    u1(&mut buf, &mut byte, &mut bit, 0);
    // bottom_field_pic_order_in_frame_present_flag = 0
    u1(&mut buf, &mut byte, &mut bit, 0);
    // num_slice_groups_minus1 = 0 (single slice group)
    ue(&mut buf, &mut byte, &mut bit, 0);
    // The rest of the PPS fields depend on num_slice_groups_minus1 == 0
    // num_ref_idx_l0_default_active_minus1 = 0
    ue(&mut buf, &mut byte, &mut bit, 0);
    // num_ref_idx_l1_default_active_minus1 = 0
    ue(&mut buf, &mut byte, &mut bit, 0);
    // weighted_pred_flag = 0
    u1(&mut buf, &mut byte, &mut bit, 0);
    // weighted_bipred_idc = 0 (2 bits)
    u1(&mut buf, &mut byte, &mut bit, 0);
    u1(&mut buf, &mut byte, &mut bit, 0);
    // pic_init_qp_minus26 = 0 (se(v) = signed, but for Baseline we use 0)
    // For simplicity, use ue(v) since the value is 0:
    ue(&mut buf, &mut byte, &mut bit, 0);
    // pic_init_qs_minus26 = 0
    ue(&mut buf, &mut byte, &mut bit, 0);
    // chroma_qp_index_offset = 0 (se(v), but value is 0 → ue(v) with 0)
    ue(&mut buf, &mut byte, &mut bit, 0);
    // deblocking_filter_control_present_flag = 0
    u1(&mut buf, &mut byte, &mut bit, 0);
    // constrained_intra_pred_flag = 0
    u1(&mut buf, &mut byte, &mut bit, 0);
    // redundant_pic_cnt_present_flag = 0
    u1(&mut buf, &mut byte, &mut bit, 0);
    // rbsp_trailing_bits
    trailing(&mut buf, &mut byte, &mut bit);

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_sps_pps_1080p() {
        let (sps, pps) = generate_sps_pps(1920, 1080, 30);

        // SPS in AVCC format: [4B len][NAL data...]
        assert!(sps.len() >= 5, "SPS should have AVCC prefix + NAL header");
        // The NAL data starts at index 4 (after the 4-byte length prefix)
        assert_eq!(sps[4] & 0x1F, 7, "SPS NAL type should be 7");
        // AVCC format: first 4 bytes are length prefix
        let sps_len = u32::from_be_bytes([sps[0], sps[1], sps[2], sps[3]]) as usize;
        assert_eq!(sps_len + 4, sps.len(), "SPS AVCC length should match");

        assert!(pps.len() >= 5, "PPS should have AVCC prefix + NAL header");
        assert_eq!(pps[4] & 0x1F, 8, "PPS NAL type should be 8");
        let pps_len = u32::from_be_bytes([pps[0], pps[1], pps[2], pps[3]]) as usize;
        assert_eq!(pps_len + 4, pps.len(), "PPS AVCC length should match");
    }

    #[test]
    fn test_generate_sps_pps_720p() {
        let (sps, pps) = generate_sps_pps(1280, 720, 60);
        assert!(!sps.is_empty());
        assert!(!pps.is_empty());
    }

    #[test]
    fn test_generate_sps_pps_4k() {
        let (sps, pps) = generate_sps_pps(3840, 2160, 30);
        assert!(!sps.is_empty());
        assert!(!pps.is_empty());
    }

    #[test]
    fn test_synthetic_sps_pps_contain_valid_nal_types() {
        // Verify the generated SPS/PPS have correct NAL unit types
        let (sps, pps) = generate_sps_pps(1920, 1080, 30);

        // AVCC format: first 4 bytes are length prefix, then the NAL data
        assert!(sps.len() >= 5, "SPS should have AVCC prefix + NAL header");
        assert!(pps.len() >= 5, "PPS should have AVCC prefix + NAL header");

        assert!(sps.len() >= 5, "SPS should have AVCC prefix + NAL header");
        assert_eq!(sps[4] & 0x1F, 7, "SPS NAL type should be 7");

        assert!(pps.len() >= 5, "PPS should have AVCC prefix + NAL header");
        assert_eq!(pps[4] & 0x1F, 8, "PPS NAL type should be 8");
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
