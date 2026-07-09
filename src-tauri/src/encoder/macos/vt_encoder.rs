//! Real-time H.264 encoder for the macOS shadow buffer using VideoToolbox.
//!
//! Mirrors [`MfH264Encoder`] on Windows — compresses raw NV12 frames into
//! H.264 AVCC packets (~10 KB each) so the ring buffer can hold minutes of
//! footage instead of seconds.

use apple_cf::iosurface::IOSurface;
use videotoolbox::compression::{CompressionSession, EncodedFrame};
use videotoolbox::session::Codec as VtCodec;

use crate::encoder::EncodeError;

/// A single compressed H.264 packet in AVCC format (4-byte length prefix).
pub struct EncodedPacket {
    pub data: Vec<u8>,
    pub is_sync: bool,
}

/// VideoToolbox-backed real-time H.264 encoder for the macOS shadow buffer.
pub struct VtH264Encoder {
    session: CompressionSession,
    surface: IOSurface,
    width: u32,
    height: u32,
    stride: usize,
    alloc_size: usize,
    frame_index: u64,
    fps: u32,
    keyframe_interval: u32,
    /// Cached SPS NAL unit in AVCC format.
    sps: Vec<u8>,
    /// Cached PPS NAL unit in AVCC format.
    pps: Vec<u8>,
    sps_pps_ready: bool,
}

// SAFETY: `CompressionSession` and `IOSurface` are both thread-safe to use
// with `&mut self` serialization, which our API guarantees.
unsafe impl Send for VtH264Encoder {}

impl VtH264Encoder {
    pub fn new(
        width: u32,
        height: u32,
        fps: u32,
        bitrate_kbps: u32,
        keyframe_interval: u32,
    ) -> Result<Self, EncodeError> {
        let stride = (width as usize) * 4;
        let alloc_size = (height as usize) * stride;

        let session =
            CompressionSession::builder(width as i32, height as i32, VtCodec::H264)
                .with_real_time(true)
                .with_average_bit_rate((bitrate_kbps as i32).saturating_mul(1000))
                .with_expected_frame_rate(fps as f64)
                .with_max_keyframe_interval(keyframe_interval as i32)
                .build()
                .map_err(|e| EncodeError::InitFailed(format!("VT CompressionSession init: {e}")))?;

        let bgra_fcc = u32::from_be_bytes(*b"BGRA");
        let surface = IOSurface::create_with_properties(
            width as usize,
            height as usize,
            bgra_fcc,
            4,
            stride,
            alloc_size,
            None,
        )
        .ok_or_else(|| {
            EncodeError::InitFailed("IOSurface::create_with_properties failed".into())
        })?;

        tracing::info!(
            "VtH264Encoder created: {}x{} stride={} alloc={} fps={} bitrate={} kbps",
            width,
            height,
            stride,
            alloc_size,
            fps,
            bitrate_kbps,
        );

        Ok(Self {
            session,
            surface,
            width,
            height,
            stride,
            alloc_size,
            frame_index: 0,
            fps,
            keyframe_interval,
            sps: Vec::new(),
            pps: Vec::new(),
            sps_pps_ready: false,
        })
    }

    /// Encode a single NV12 frame into compressed H.264 AVCC packet(s).
    ///
    /// Returns one packet per frame on success. Falls back to the caller
    /// if encoding produces no data (encoder not ready, etc).
    pub fn encode_frame(&mut self, nv12_data: &[u8], width: u32, height: u32) -> Result<Vec<EncodedPacket>, EncodeError> {
        // --- Step 1: NV12 → BGRA (VT expects BGRA IOSurface) ---
        let bgra = crate::capture::nv12_to_bgra(nv12_data, width, height);

        // --- Step 2: Copy BGRA into the IOSurface ---
        let mut guard = self
            .surface
            .lock_read_write()
            .map_err(|e| EncodeError::EncodeFailed(format!("IOSurface lock: {e}")))?;

        let dst_base = guard
            .base_address_mut()
            .ok_or_else(|| EncodeError::EncodeFailed("IOSurface base address unavailable".into()))?;

        let row_bytes = (self.width as usize) * 4;
        let copy_height = self.height as usize;

        // If the source dimensions differ from the encoder target dimensions,
        // resize the BGRA data inline before copying.
        let resized;
        let src: &[u8] = if width == self.width && height == self.height {
            &bgra
        } else {
            let src_stride = (width as usize) * 4;
            resized = crate::encoder::macos::resize_bgra_frame(
                &bgra,
                width,
                height,
                self.width,
                self.height,
                src_stride,
            )?;
            &resized
        };

        for y in 0..copy_height {
            let src_off = y * row_bytes;
            let dst_off = y * row_bytes;
            if src_off + row_bytes <= src.len() && dst_off + row_bytes <= self.alloc_size {
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        src.as_ptr().add(src_off),
                        dst_base.add(dst_off),
                        row_bytes,
                    );
                }
            }
        }
        drop(guard);

        // --- Step 3: Encode via VideoToolbox ---
        let timescale = self.fps as i32;
        let presentation_time = (self.frame_index as i64, timescale);

        let encoded: EncodedFrame = self
            .session
            .encode(&self.surface, presentation_time)
            .map_err(|e| EncodeError::EncodeFailed(format!("VT encode frame {}: {e}", self.frame_index)))?;

        self.frame_index += 1;

        // --- Step 4: Extract and cache SPS/PPS from the first keyframe ---
        // VideoToolbox exposes SPS/PPS via the CMSampleBuffer's format
        // description, not necessarily in the encoded bitstream. Use the
        // format description to get raw NAL units, then wrap them with
        // 4-byte AVCC length prefixes to match MfH264Encoder's format.
        if !encoded.data.is_empty() && !self.sps_pps_ready {
            match crate::encoder::macos::extract_h264_parameter_sets(&encoded) {
                Ok((raw_sps, raw_pps)) => {
                    let mut sps_avcc = Vec::with_capacity(4 + raw_sps.len());
                    sps_avcc.extend_from_slice(&(raw_sps.len() as u32).to_be_bytes());
                    sps_avcc.extend_from_slice(&raw_sps);

                    let mut pps_avcc = Vec::with_capacity(4 + raw_pps.len());
                    pps_avcc.extend_from_slice(&(raw_pps.len() as u32).to_be_bytes());
                    pps_avcc.extend_from_slice(&raw_pps);

                    self.sps = sps_avcc;
                    self.pps = pps_avcc;
                    self.sps_pps_ready = true;
                    tracing::info!(
                        "VtH264Encoder: captured SPS({}) PPS({}) from format description",
                        self.sps.len(),
                        self.pps.len(),
                    );
                }
                Err(e) => {
                    tracing::warn!("VtH264Encoder: failed to extract SPS/PPS from format description: {e}");
                }
            }
        }

        // --- Step 5: Wrap into EncodedPacket(s) ---
        // VT typically produces one EncodedFrame per input frame.
        // If the data is empty, the encoder may have skipped the frame.
        if encoded.data.is_empty() {
            return Ok(Vec::new());
        }

        let is_sync = self.sps_pps_ready
            && (self.keyframe_interval == 0
                || (self.frame_index - 1) % self.keyframe_interval as u64 == 0);

        Ok(vec![EncodedPacket {
            data: encoded.data,
            is_sync,
        }])
    }

    pub fn sps_pps_ready(&self) -> bool {
        self.sps_pps_ready
    }

    pub fn sps(&self) -> &[u8] {
        &self.sps
    }

    pub fn pps(&self) -> &[u8] {
        &self.pps
    }

    /// Reset frame index (call on recording restart).
    pub fn reset(&mut self) {
        self.frame_index = 0;
        self.sps.clear();
        self.pps.clear();
        self.sps_pps_ready = false;
    }
}

/// Scan AVCC-format data for SPS (NAL type 7) and PPS (NAL type 8).
/// Assumes 4-byte big-endian length prefixes per NAL unit.
fn extract_nal_sps_pps(data: &[u8]) -> Result<(Vec<u8>, Vec<u8>), EncodeError> {
    let mut offset = 0;
    let mut sps = Vec::new();
    let mut pps = Vec::new();

    while offset + 4 <= data.len() {
        let nal_len = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if offset + nal_len > data.len() {
            break;
        }
        let nal_type = data[offset] & 0x1F;
        let nal_data = data[offset..offset + nal_len].to_vec();
        match nal_type {
            7 => sps = nal_data,
            8 => pps = nal_data,
            _ => {}
        }
        offset += nal_len;
    }

    if !sps.is_empty() && !pps.is_empty() {
        Ok((sps, pps))
    } else {
        Err(EncodeError::EncodeFailed(
            "SPS/PPS not found in encoded frame".into(),
        ))
    }
}
