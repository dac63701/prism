//! macOS hardware encoder using VideoToolbox.
//!
//! Uses the [`videotoolbox`] crate for VTCompressionSession (HW H.264/HEVC)
//! and the [`mp4`] crate for ISO-BMFF muxing.

pub mod vt_encoder;

use std::path::Path;

use apple_cf::iosurface::IOSurface;
use bytes::Bytes;
use image::{imageops::FilterType, ImageBuffer, Rgba};
use videotoolbox::compression::{CompressionSession, EncodedFrame};
use videotoolbox::session::Codec as VtCodec;

use crate::buffer::StoredFrame;
use crate::capture::PixelFormat;
use crate::encoder::codecs::{Codec, EncoderConfig};
use crate::encoder::{extract_sps_pps, EncodeError, Encoder};

/// macOS VideoToolbox-backed hardware encoder.
pub struct MacEncoder;

impl MacEncoder {
    pub fn new() -> Self {
        Self
    }
}

impl Encoder for MacEncoder {
    fn encode_clip(
        &mut self,
        frames: &[StoredFrame],
        output_path: &Path,
        config: &EncoderConfig,
    ) -> Result<(), EncodeError> {
        if frames.is_empty() {
            return Err(EncodeError::EncodeFailed("No frames to encode".into()));
        }

        // Fast path: frames are already H.264 AVCC — mux directly.
        if frames[0].pixel_format == PixelFormat::H264 {
            return mux_h264_clip(frames, output_path, config);
        }

        let source_width = frames[0].width;
        let source_height = frames[0].height;
        let target_width = config.target_width.max(1);
        let target_height = config.target_height.max(1);
        let stride = (target_width as usize) * 4;

        tracing::info!(
            "Encoding {} frames (source={}x{}, target={}x{}, stride={}, fps={}, codec={:?}, bitrate={} kbps)",
            frames.len(),
            source_width,
            source_height,
            target_width,
            target_height,
            stride,
            config.fps,
            config.codec,
            config.bitrate_kbps,
        );

        // Map our Codec to videotoolbox Codec
        let vt_codec = match config.codec {
            Codec::H264 => VtCodec::H264,
            Codec::H265 => VtCodec::HEVC,
            Codec::Av1 => {
                return Err(EncodeError::UnsupportedCodec(
                    "AV1 encoding not supported via VideoToolbox on this macOS version".into(),
                ))
            }
        };

        // Build the compression session
        let session =
            CompressionSession::builder(target_width as i32, target_height as i32, vt_codec)
                .with_real_time(true)
                .with_average_bit_rate((config.bitrate_kbps as i32).saturating_mul(1000))
                .with_expected_frame_rate(config.fps as f64)
                .with_max_keyframe_interval(config.keyframe_interval as i32)
                .build()
                .map_err(|e| EncodeError::InitFailed(format!("VideoToolbox init: {e}")))?;

        // Create a single IOSurface reused for all frames.
        let bgra_fcc = u32::from_be_bytes(*b"BGRA");
        let alloc_size = (target_height as usize) * stride;

        let surface = IOSurface::create_with_properties(
            target_width as usize,
            target_height as usize,
            bgra_fcc,
            4, // bytes per element
            stride,
            alloc_size,
            None,
        )
        .ok_or_else(|| {
            EncodeError::InitFailed("IOSurface::create_with_properties failed".into())
        })?;

        tracing::info!(
            "IOSurface created: {}x{} stride={} size={}",
            target_width,
            target_height,
            stride,
            alloc_size
        );

        // Encode each frame
        let timescale = crate::encoder::MP4_TIMESCALE as i32;
        let timing = crate::encoder::mp4_sample_timing(frames, config.fps);
        let mut encoded_samples: Vec<EncodedFrame> = Vec::with_capacity(frames.len());

        for (i, frame) in frames.iter().enumerate() {
            // Step 1: Convert NV12 → BGRA if needed (ring buffer stores NV12 on macOS)
            let bgra_buf;
            let bgra_data: &[u8] = match frame.pixel_format {
                crate::capture::PixelFormat::Nv12 => {
                    bgra_buf = crate::capture::nv12_to_bgra(
                        frame.data.as_slice(),
                        frame.width,
                        frame.height,
                    );
                    &bgra_buf
                }
                _ => frame.data.as_slice(),
            };

            // Step 2: Resize if the source doesn't match target dimensions
            let src_stride = (frame.width as usize) * 4; // BGRA is tightly packed after conversion
            let resized_bgra;
            let src: &[u8] = if frame.width == target_width
                && frame.height == target_height
                && src_stride == stride
            {
                bgra_data
            } else {
                resized_bgra = resize_bgra_frame(
                    bgra_data,
                    frame.width,
                    frame.height,
                    target_width,
                    target_height,
                    src_stride,
                )?;
                &resized_bgra
            };

            // Lock the IOSurface and copy the frame data into it
            let mut guard = surface
                .lock_read_write()
                .map_err(|e| EncodeError::EncodeFailed(format!("IOSurface lock frame {i}: {e}")))?;

            let dst_base = guard.base_address_mut().ok_or_else(|| {
                EncodeError::EncodeFailed("IOSurface base address unavailable".into())
            })?;

            let row_stride = stride;
            let copy_height = target_height as usize;
            let copy_bytes_per_row = stride;

            for y in 0..copy_height {
                let src_off = y * row_stride;
                let dst_off = y * row_stride;
                let to_copy = copy_bytes_per_row;
                if src_off + to_copy <= src.len() && dst_off + to_copy <= alloc_size {
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            src.as_ptr().add(src_off),
                            dst_base.add(dst_off),
                            to_copy,
                        );
                    }
                }
            }

            drop(guard);

            // Encode via the crate's method — it wraps our IOSurface into a
            // CVPixelBuffer and passes it to VTCompressionSessionEncodeFrame.
            let presentation_time = (timing[i].0 as i64, timescale);
            let encoded = session
                .encode(&surface, presentation_time)
                .map_err(|e| EncodeError::EncodeFailed(format!("VT encode frame {i}: {e}")))?;

            encoded_samples.push(encoded);
        }

        // The `videotoolbox` crate already returns the encoded sample bytes in
        // the container-friendly length-prefixed format. Do not re-parse them
        // as Annex B or the bitstream gets mangled into an invalid file.
        let written_samples: Vec<&EncodedFrame> = encoded_samples
            .iter()
            .filter(|sample| !sample.data.is_empty())
            .collect();

        let first_sample = written_samples
            .first()
            .copied()
            .ok_or_else(|| EncodeError::EncodeFailed("No encoded sample data produced".into()))?;

        let (sps, pps) = extract_h264_parameter_sets(first_sample)
            .map_err(|e| EncodeError::EncodeFailed(format!("H.264 config: {e}")))?;

        // Build MP4 container
        use mp4::{AvcConfig, MediaConfig, Mp4Config, Mp4Writer, TrackConfig, TrackType};

        let mp4_config = Mp4Config {
            major_brand: "isom".parse().unwrap(),
            minor_version: 512,
            compatible_brands: vec![
                "isom".parse().unwrap(),
                "iso2".parse().unwrap(),
                "avc1".parse().unwrap(),
            ],
            timescale: timescale as u32,
        };

        let file = std::fs::File::create(output_path)
            .map_err(|e| EncodeError::OutputFailed(format!("Failed to create output file: {e}")))?;

        let mut writer = Mp4Writer::write_start(file, &mp4_config)
            .map_err(|e| EncodeError::OutputFailed(format!("Mp4Writer start: {e}")))?;

        // AVC/H.264 video track with SPS/PPS
        let avc_config = AvcConfig {
            width: target_width as u16,
            height: target_height as u16,
            seq_param_set: sps,
            pic_param_set: pps,
        };

        let track_config = TrackConfig {
            track_type: TrackType::Video,
            timescale: timescale as u32,
            language: "und".to_string(),
            media_conf: MediaConfig::AvcConfig(avc_config),
        };

        writer
            .add_track(&track_config)
            .map_err(|e| EncodeError::OutputFailed(format!("add_track: {e}")))?;

        // Write samples with actual capture timestamps.
        // Map written_samples back to their original frame indices.
        let mut written_idx = 0usize;
        for (frame_idx, frame) in frames.iter().enumerate() {
            // Find the corresponding encoded sample for this original frame.
            let has_output =
                frame_idx < encoded_samples.len() && !encoded_samples[frame_idx].data.is_empty();
            if !has_output {
                continue;
            }
            let sample = &encoded_samples[frame_idx];

            let start_time = timing[frame_idx].0;
            let duration = {
                let next_delta = if written_idx + 1 < written_samples.len() {
                    // Find the next original frame index that has output
                    let mut next_idx = frame_idx + 1;
                    while next_idx < frames.len() && encoded_samples[next_idx].data.is_empty() {
                        next_idx += 1;
                    }
                    if next_idx < frames.len() {
                        frames[next_idx]
                            .timestamp
                            .saturating_duration_since(frame.timestamp)
                    } else {
                        std::time::Duration::from_micros(timing[frame_idx].1 as u64)
                    }
                } else {
                    std::time::Duration::from_micros(timing[frame_idx].1 as u64)
                };
                (next_delta.as_micros() as u64).max(1)
            };

            let is_sync = written_idx == 0
                || (config.keyframe_interval > 0
                    && (written_idx as u32).is_multiple_of(config.keyframe_interval));

            let mp4_sample = mp4::Mp4Sample {
                start_time,
                duration: duration.min(u64::from(u32::MAX)) as u32,
                rendering_offset: 0,
                is_sync,
                bytes: Bytes::copy_from_slice(&sample.data),
            };

            writer
                .write_sample(1, &mp4_sample)
                .map_err(|e| EncodeError::EncodeFailed(format!("write_sample {frame_idx}: {e}")))?;

            written_idx += 1;
        }

        // Finalize MP4
        writer
            .write_end()
            .map_err(|e| EncodeError::OutputFailed(format!("Mp4Writer end: {e}")))?;

        Ok(())
    }
}

pub(crate) fn extract_h264_parameter_sets(
    sample: &EncodedFrame,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let sample_buffer = sample
        .cm_sample_buffer()
        .ok_or_else(|| "encoded sample has no CMSampleBuffer".to_string())?;
    let format_description = sample_buffer
        .format_description()
        .ok_or_else(|| "encoded sample has no format description".to_string())?;

    if !format_description.is_video() {
        return Err("sample format description is not video".into());
    }

    let mut sps = Vec::new();
    let mut pps = Vec::new();

    unsafe {
        let mut param_count: usize = 0;
        let mut nal_header_len: i32 = 0;

        let mut param_ptr: *const u8 = std::ptr::null();
        let mut param_size: usize = 0;
        let status = apple_cf::raw::CMVideoFormatDescriptionGetH264ParameterSetAtIndex(
            format_description.as_ptr().cast(),
            0,
            &mut param_ptr,
            &mut param_size,
            &mut param_count,
            &mut nal_header_len,
        );
        if status != 0 || param_ptr.is_null() || param_size == 0 {
            return Err(format!("failed to read H.264 SPS: status={status}"));
        }
        sps.extend_from_slice(std::slice::from_raw_parts(param_ptr, param_size));

        param_ptr = std::ptr::null();
        param_size = 0;
        let status = apple_cf::raw::CMVideoFormatDescriptionGetH264ParameterSetAtIndex(
            format_description.as_ptr().cast(),
            1,
            &mut param_ptr,
            &mut param_size,
            &mut param_count,
            &mut nal_header_len,
        );
        if status != 0 || param_ptr.is_null() || param_size == 0 {
            return Err(format!("failed to read H.264 PPS: status={status}"));
        }
        pps.extend_from_slice(std::slice::from_raw_parts(param_ptr, param_size));
    }

    Ok((sps, pps))
}

/// Mux pre-encoded H.264 AVCC frames directly into an MP4 container.
/// Matches [`WindowsEncoder::encode_clip`] — no re-encoding.
fn mux_h264_clip(
    frames: &[StoredFrame],
    output_path: &Path,
    config: &EncoderConfig,
) -> Result<(), EncodeError> {
    let (sps, pps) = extract_sps_pps(frames)?;

    use mp4::{AvcConfig, MediaConfig, Mp4Config, Mp4Writer, TrackConfig, TrackType};

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

    for (i, frame) in frames.iter().enumerate() {
        if frame.pixel_format != PixelFormat::H264 {
            return Err(EncodeError::EncodeFailed(
                "mux_h264_clip expects H.264 encoded frames".into(),
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

pub(crate) fn resize_bgra_frame(
    data: &[u8],
    width: u32,
    height: u32,
    target_width: u32,
    target_height: u32,
    stride: usize,
) -> Result<Vec<u8>, EncodeError> {
    let mut rgba = Vec::with_capacity((width as usize) * (height as usize) * 4);
    for y in 0..height {
        let row_start = (y as usize) * stride;
        for x in 0..width {
            let off = row_start + (x as usize) * 4;
            rgba.extend_from_slice(&[data[off + 2], data[off + 1], data[off], data[off + 3]]);
        }
    }

    let image = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(width, height, rgba)
        .ok_or_else(|| EncodeError::EncodeFailed("Failed to build source image buffer".into()))?;
    let resized =
        image::imageops::resize(&image, target_width, target_height, FilterType::Triangle);

    let mut bgra = Vec::with_capacity((target_width as usize) * (target_height as usize) * 4);
    for pixel in resized.pixels() {
        bgra.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
    }

    Ok(bgra)
}
