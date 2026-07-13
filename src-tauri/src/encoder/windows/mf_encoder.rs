//! Media Foundation H.264 hardware encoder wrapper.
//!
//! Wraps the H.264 Video Encoder MFT (NVENC / AMF / QuickSync depending on GPU)
//! to produce compressed H.264 packets from NV12 input frames.
//!
//! The encoder outputs H.264 Annex B byte-stream which is converted to AVCC
//! (4-byte length-prefix format) suitable for direct MP4 muxing.

use std::sync::OnceLock;

use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::*;

use crate::encoder::EncodeError;

// ---------------------------------------------------------------------------
// One-shot MF startup
// ---------------------------------------------------------------------------

static MF_INITIALIZED: OnceLock<Result<(), windows::core::Error>> = OnceLock::new();

/// Pack Media Foundation size and ratio attributes: the first value occupies
/// the high 32 bits and the second value occupies the low 32 bits.
fn pack_mf_attribute_pair(first: u32, second: u32) -> u64 {
    ((first as u64) << 32) | second as u64
}

fn ensure_mf() -> Result<(), EncodeError> {
    let result =
        MF_INITIALIZED.get_or_init(|| unsafe { MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET) });
    match result {
        Ok(()) => Ok(()),
        Err(e) => Err(EncodeError::InitFailed(format!("MFStartup failed: {e}"))),
    }
}

// ---------------------------------------------------------------------------
// Encoded packet
// ---------------------------------------------------------------------------

/// A single compressed H.264 NAL unit or combined packet in AVCC format.
pub struct EncodedPacket {
    pub data: Vec<u8>,
    pub is_sync: bool,
}

// ---------------------------------------------------------------------------
// H.264 encoder
// ---------------------------------------------------------------------------

/// Media Foundation H.264 Video Encoder MFT wrapper.
pub struct MfH264Encoder {
    transform: IMFTransform,
    frame_index: i64,
    timescale: i64,
    width: u32,
    height: u32,
    /// Cached SPS NAL unit in AVCC format (4-byte length prefix).
    sps: Vec<u8>,
    /// Cached PPS NAL unit in AVCC format (4-byte length prefix).
    pps: Vec<u8>,
    /// Whether sps/pps have been populated at least once.
    sps_pps_ready: bool,
}

// SAFETY: `IMFTransform` is a COM pointer. The underlying MFT (H.264 Video Encoder)
// supports serialized calls from a single thread at a time, which is how we use it.
// COM apartment management is handled by `CoInitializeEx` (called elsewhere in the
// app via `MFStartup`). The pointer is safe to move between threads as long as
// calls are serialized, which our `&mut self` API guarantees.
unsafe impl Send for MfH264Encoder {}

impl MfH264Encoder {
    /// Create a new H.264 encoder with the given parameters.
    ///
    /// * `width`, `height` — video dimensions (input must be NV12 at this res)
    /// * `fps` — frame rate
    /// * `bitrate_kbps` — target average bitrate in kilobits / sec
    /// * `keyframe_interval` — GOP size (keyframe every N frames)
    pub fn new(
        width: u32,
        height: u32,
        fps: u32,
        bitrate_kbps: u32,
        keyframe_interval: u32,
    ) -> Result<Self, EncodeError> {
        ensure_mf()?;

        unsafe {
            // Create the H.264 encoder MFT via its CLSID
            let transform: IMFTransform =
                CoCreateInstance(&CLSID_MSH264EncoderMFT, None, CLSCTX_INPROC_SERVER).map_err(
                    |e| EncodeError::InitFailed(format!("CoCreateInstance H.264 encoder MFT: {e}")),
                )?;

            // ------ Set input type: NV12 ------
            let input_type: IMFMediaType = MFCreateMediaType()
                .map_err(|e| EncodeError::InitFailed(format!("MFCreateMediaType input: {e}")))?;

            input_type
                .SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
                .map_err(|e| EncodeError::InitFailed(format!("SetGUID major: {e}")))?;
            input_type
                .SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_NV12)
                .map_err(|e| EncodeError::InitFailed(format!("SetGUID subtype: {e}")))?;
            input_type
                .SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)
                .map_err(|e| EncodeError::InitFailed(format!("SetUINT32 interlace: {e}")))?;

            let frame_size = pack_mf_attribute_pair(width, height);
            input_type
                .SetUINT64(&MF_MT_FRAME_SIZE, frame_size)
                .map_err(|e| EncodeError::InitFailed(format!("SetUINT64 frame size: {e}")))?;

            let frame_rate = pack_mf_attribute_pair(fps, 1);
            input_type
                .SetUINT64(&MF_MT_FRAME_RATE, frame_rate)
                .map_err(|e| EncodeError::InitFailed(format!("SetUINT64 frame rate: {e}")))?;

            let pixel_aspect: u64 = (1u64) << 32 | 1u64;
            input_type
                .SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pixel_aspect)
                .ok();

            // ------ Set output type: H.264 ------
            // The Microsoft encoder requires these attributes to configure its
            // H.264 output; the advertised type omits them until after setup.
            let output_type: IMFMediaType = MFCreateMediaType()
                .map_err(|e| EncodeError::InitFailed(format!("MFCreateMediaType output: {e}")))?;

            output_type
                .SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
                .map_err(|e| EncodeError::InitFailed(format!("SetGUID major out: {e}")))?;
            output_type
                .SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264)
                .map_err(|e| EncodeError::InitFailed(format!("SetGUID subtype out: {e}")))?;
            output_type
                .SetUINT32(&MF_MT_AVG_BITRATE, bitrate_kbps.saturating_mul(1_000))
                .map_err(|e| EncodeError::InitFailed(format!("SetUINT32 bitrate: {e}")))?;
            output_type
                .SetUINT64(&MF_MT_FRAME_SIZE, frame_size)
                .map_err(|e| EncodeError::InitFailed(format!("SetUINT64 frame size out: {e}")))?;
            output_type
                .SetUINT64(&MF_MT_FRAME_RATE, frame_rate)
                .map_err(|e| EncodeError::InitFailed(format!("SetUINT64 frame rate out: {e}")))?;
            output_type
                .SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)
                .map_err(|e| EncodeError::InitFailed(format!("SetUINT32 interlace out: {e}")))?;
            output_type
                .SetUINT32(&MF_MT_MPEG2_PROFILE, 66)
                .map_err(|e| EncodeError::InitFailed(format!("SetUINT32 profile: {e}")))?;
            output_type
                .SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pixel_aspect)
                .map_err(|e| EncodeError::InitFailed(format!("SetUINT64 pixel aspect out: {e}")))?;
            // Bound the GOP so a clip can begin from a nearby sync frame.
            // Otherwise the MFT chooses its own interval and clip extraction
            // may need to discard several seconds before the next keyframe.
            output_type
                .SetUINT32(&MF_MT_MAX_KEYFRAME_SPACING, keyframe_interval.max(1))
                .map_err(|e| EncodeError::InitFailed(format!("SetUINT32 keyframe spacing: {e}")))?;

            transform
                .SetOutputType(0, &output_type, 0)
                .map_err(|e| EncodeError::InitFailed(format!("SetOutputType: {e}")))?;

            // The Windows H.264 encoder declares the input stream dependent on
            // the output stream. Negotiate H.264 output first; otherwise it
            // returns MF_E_TRANSFORM_TYPE_NOT_SET from SetInputType.
            transform
                .SetInputType(0, &input_type, 0)
                .map_err(|e| EncodeError::InitFailed(format!("SetInputType: {e}")))?;

            transform
                .ProcessMessage(MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, 0)
                .map_err(|e| EncodeError::InitFailed(format!("Begin streaming: {e}")))?;
            transform
                .ProcessMessage(MFT_MESSAGE_NOTIFY_START_OF_STREAM, 0)
                .map_err(|e| EncodeError::InitFailed(format!("Start stream: {e}")))?;

            Ok(Self {
                transform,
                frame_index: 0,
                timescale: fps as i64,
                width,
                height,
                sps: Vec::new(),
                pps: Vec::new(),
                sps_pps_ready: false,
            })
        }
    }

    /// Encode a single NV12 frame.
    ///
    /// Returns zero or more [`EncodedPacket`] — usually one per frame, but the
    /// MFT may batch multiple frames internally before producing output.
    pub fn encode_frame(&mut self, nv12: &[u8]) -> Result<Vec<EncodedPacket>, EncodeError> {
        unsafe {
            let expected_size = (self.width * self.height * 3 / 2) as usize;
            if nv12.len() < expected_size {
                return Err(EncodeError::EncodeFailed(format!(
                    "NV12 buffer too small: got {} expected {}",
                    nv12.len(),
                    expected_size
                )));
            }

            // ------ Create input sample ------
            let buffer: IMFMediaBuffer = MFCreateMemoryBuffer(nv12.len() as u32)
                .map_err(|e| EncodeError::EncodeFailed(format!("CreateMemoryBuffer: {e}")))?;

            let mut ptr: *mut u8 = std::ptr::null_mut();
            let mut max_len: u32 = 0;
            let mut cur_len: u32 = 0;

            buffer
                .Lock(&mut ptr, Some(&mut max_len), Some(&mut cur_len))
                .map_err(|e| EncodeError::EncodeFailed(format!("Lock buffer: {e}")))?;

            std::ptr::copy_nonoverlapping(nv12.as_ptr(), ptr, nv12.len());

            buffer
                .SetCurrentLength(nv12.len() as u32)
                .map_err(|e| EncodeError::EncodeFailed(format!("SetCurrentLength: {e}")))?;

            buffer
                .Unlock()
                .map_err(|e| EncodeError::EncodeFailed(format!("Unlock buffer: {e}")))?;

            let sample: IMFSample = MFCreateSample()
                .map_err(|e| EncodeError::EncodeFailed(format!("CreateSample: {e}")))?;

            sample
                .AddBuffer(&buffer)
                .map_err(|e| EncodeError::EncodeFailed(format!("AddBuffer: {e}")))?;

            let duration_100ns = 10_000_000 / self.timescale;
            let timestamp_100ns = self.frame_index * duration_100ns;

            sample
                .SetSampleTime(timestamp_100ns)
                .map_err(|e| EncodeError::EncodeFailed(format!("SetSampleTime: {e}")))?;
            sample
                .SetSampleDuration(duration_100ns)
                .map_err(|e| EncodeError::EncodeFailed(format!("SetSampleDuration: {e}")))?;

            // Mark first frame as sync point
            if self.frame_index == 0 {
                sample.SetUINT32(&MFSampleExtension_CleanPoint, 1).ok();
            }

            // ------ ProcessInput ------
            self.transform
                .ProcessInput(0, &sample, 0)
                .map_err(|e| EncodeError::EncodeFailed(format!("ProcessInput: {e}")))?;

            // ------ ProcessOutput (may produce 0 or more samples) ------
            let mut packets: Vec<EncodedPacket> = Vec::new();

            loop {
                let mut output = self.create_output_buffer()?;
                let mut status: u32 = 0;

                let result =
                    self.transform
                        .ProcessOutput(0, std::slice::from_mut(&mut output), &mut status);

                if result.is_ok() {
                    let packet_result = (|| -> Result<Option<EncodedPacket>, EncodeError> {
                        let Some(ref out_sample) = *output.pSample else {
                            return Ok(None);
                        };
                        let raw = collect_sample_bytes(out_sample)?;
                        let avcc = h264_to_avcc(&raw)?;
                        let clean_point = is_keyframe(out_sample);
                        let has_idr = avcc_contains_idr(&avcc);
                        let is_key = clean_point || has_idr;

                        if has_idr && !clean_point {
                            eprintln!(
                                "[prism] marked H.264 packet as sync from its IDR NAL (MFT omitted CleanPoint)"
                            );
                        }

                        // MFTs may return either Annex B or AVCC samples. Normalize
                        // first so both packet storage and parameter-set parsing use
                        // the AVCC format required by the MP4 muxer.
                        if !self.sps_pps_ready {
                            capture_sps_pps_from_avcc(&avcc, &mut self.sps, &mut self.pps);
                            if !self.sps.is_empty() && !self.pps.is_empty() {
                                self.sps_pps_ready = true;
                            }
                        }

                        Ok(Some(EncodedPacket {
                            data: avcc,
                            is_sync: is_key,
                        }))
                    })();
                    release_output_buffer(&mut output);
                    if let Some(packet) = packet_result? {
                        packets.push(packet);
                    }
                } else if let Err(err) = &result {
                    if err.code() == MF_E_TRANSFORM_NEED_MORE_INPUT {
                        // Normal encoder latency: retain input internally until
                        // enough samples are available to produce output.
                        release_output_buffer(&mut output);
                        break;
                    }
                    if err.code() == MF_E_TRANSFORM_STREAM_CHANGE {
                        // Renegotiate the output type requested by the MFT.
                        if let Ok(new_type) = self.transform.GetOutputAvailableType(0, 0) {
                            self.transform.SetOutputType(0, &new_type, 0).ok();
                        }
                        release_output_buffer(&mut output);
                        continue;
                    }
                    release_output_buffer(&mut output);
                    return Err(EncodeError::EncodeFailed(format!("ProcessOutput: {err}")));
                }
                if result.is_ok() {
                    continue;
                }
            }

            // Fallback: if SPS/PPS still not found in the bitstream, try
            // extracting them from the output media type. Some MFT
            // implementations (certain GPU drivers) omit SPS/PPS from the
            // encoded bitstream but provide them via MF_MT_MPEG_SEQUENCE_HEADER.
            if !self.sps_pps_ready {
                if let Ok(()) =
                    capture_sps_pps_from_media_type(&self.transform, &mut self.sps, &mut self.pps)
                {
                    if !self.sps.is_empty() && !self.pps.is_empty() {
                        self.sps_pps_ready = true;
                        eprintln!(
                            "[prism] captured SPS({}) PPS({}) from output media type",
                            self.sps.len(),
                            self.pps.len()
                        );
                    }
                }
            }

            self.frame_index += 1;
            Ok(packets)
        }
    }

    /// Create the output descriptor required by this MFT. Some encoders provide
    /// their own samples; others require a caller-allocated media buffer.
    unsafe fn create_output_buffer(&self) -> Result<MFT_OUTPUT_DATA_BUFFER, EncodeError> {
        let info = self
            .transform
            .GetOutputStreamInfo(0)
            .map_err(|e| EncodeError::EncodeFailed(format!("GetOutputStreamInfo: {e}")))?;
        let provider_flags =
            (MFT_OUTPUT_STREAM_PROVIDES_SAMPLES.0 | MFT_OUTPUT_STREAM_CAN_PROVIDE_SAMPLES.0) as u32;

        if info.dwFlags & provider_flags != 0 {
            return Ok(MFT_OUTPUT_DATA_BUFFER {
                dwStreamID: 0,
                ..Default::default()
            });
        }

        let sample: IMFSample = MFCreateSample()
            .map_err(|e| EncodeError::EncodeFailed(format!("Create output sample: {e}")))?;
        let buffer: IMFMediaBuffer = MFCreateMemoryBuffer(info.cbSize.max(1))
            .map_err(|e| EncodeError::EncodeFailed(format!("Create output buffer: {e}")))?;
        sample
            .AddBuffer(&buffer)
            .map_err(|e| EncodeError::EncodeFailed(format!("Add output buffer: {e}")))?;

        Ok(MFT_OUTPUT_DATA_BUFFER {
            dwStreamID: 0,
            pSample: std::mem::ManuallyDrop::new(Some(sample)),
            ..Default::default()
        })
    }
}

/// The generated bindings use `ManuallyDrop` for COM pointers in this FFI
/// struct, so release them explicitly after every `ProcessOutput` call.
unsafe fn release_output_buffer(output: &mut MFT_OUTPUT_DATA_BUFFER) {
    std::mem::ManuallyDrop::drop(&mut output.pSample);
    std::mem::ManuallyDrop::drop(&mut output.pEvents);
}

// ---------------------------------------------------------------------------
// Helper: collect bytes from an output sample
// ---------------------------------------------------------------------------

unsafe fn collect_sample_bytes(sample: &IMFSample) -> Result<Vec<u8>, EncodeError> {
    let buffer: IMFMediaBuffer = sample
        .GetBufferByIndex(0)
        .map_err(|e| EncodeError::EncodeFailed(format!("GetBufferByIndex: {e}")))?;

    let mut ptr: *mut u8 = std::ptr::null_mut();
    let mut max_len: u32 = 0;
    let mut cur_len: u32 = 0;

    buffer
        .Lock(&mut ptr, Some(&mut max_len), Some(&mut cur_len))
        .map_err(|e| EncodeError::EncodeFailed(format!("Lock output: {e}")))?;

    let data = std::slice::from_raw_parts(ptr, cur_len as usize).to_vec();

    buffer.Unlock().ok();

    Ok(data)
}

// ---------------------------------------------------------------------------
// Helper: check if a sample is a keyframe
// ---------------------------------------------------------------------------

fn is_keyframe(sample: &IMFSample) -> bool {
    unsafe {
        if let Ok(value) = sample.GetUINT32(&MFSampleExtension_CleanPoint) {
            value != 0
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Annex B → AVCC conversion
// ---------------------------------------------------------------------------

/// Convert H.264 Annex B byte-stream (start-code delimited) to AVCC
/// (4-byte length-prefix format), as required by the `mp4` crate.
///
/// Annex B:   `00 00 00 01` or `00 00 01` prefix before each NAL
/// AVCC:      `NN NN NN NN` big-endian length before each NAL (no start code)
fn annex_b_to_avcc(annex_b: &[u8]) -> Vec<u8> {
    let mut avcc = Vec::with_capacity(annex_b.len());

    let mut i = 0;
    while i < annex_b.len() {
        // Find start code: 0x00000001 or 0x000001
        if i + 4 <= annex_b.len()
            && annex_b[i] == 0
            && annex_b[i + 1] == 0
            && annex_b[i + 2] == 0
            && annex_b[i + 3] == 1
        {
            i += 4;
        } else if i + 3 <= annex_b.len()
            && annex_b[i] == 0
            && annex_b[i + 1] == 0
            && annex_b[i + 2] == 1
        {
            i += 3;
        } else {
            i += 1;
            continue;
        };

        // Find the next start code (or end of data)
        let nal_start = i;
        while i < annex_b.len() {
            if i + 4 <= annex_b.len()
                && annex_b[i] == 0
                && annex_b[i + 1] == 0
                && annex_b[i + 2] == 0
                && annex_b[i + 3] == 1
            {
                break;
            }
            if i + 3 <= annex_b.len()
                && annex_b[i] == 0
                && annex_b[i + 1] == 0
                && annex_b[i + 2] == 1
            {
                break;
            }
            i += 1;
        }

        let nal_data = &annex_b[nal_start..i];
        if !nal_data.is_empty() {
            // Write 4-byte big-endian length
            avcc.extend_from_slice(&(nal_data.len() as u32).to_be_bytes());
            avcc.extend_from_slice(nal_data);
        }
    }

    avcc
}

/// Quick O(1) check: does the data start with an Annex B start code?
/// Annex B: `00 00 00 01` or `00 00 01` prefix before first NAL.
/// AVCC: 4-byte big-endian length prefix (first byte is rarely 0 for SPS).
fn looks_like_annex_b(data: &[u8]) -> bool {
    if data.len() < 3 {
        return false;
    }
    if data[0] != 0 || data[1] != 0 {
        return false;
    }
    data[2] == 1 || (data.len() >= 4 && data[2] == 0 && data[3] == 1)
}

/// Normalize a Media Foundation H.264 packet to AVCC. Hardware MFTs may emit
/// either Annex B or AVCC depending on the driver and negotiated output type.
///
/// Fast path: avoids the O(n) `is_valid_avcc` scan for the common case where
/// the MFT already outputs AVCC (modern drivers).
fn h264_to_avcc(data: &[u8]) -> Result<Vec<u8>, EncodeError> {
    if !looks_like_annex_b(data) {
        // Common case: MFT outputs AVCC directly — skip O(n) validation scan.
        // Still verify the first NALU length is within bounds (O(1)) to catch
        // clearly malformed packets without scanning the entire buffer.
        if data.len() >= 4 {
            let first_len = u32::from_be_bytes(data[0..4].try_into().unwrap()) as usize;
            if first_len == 0 || 4 + first_len > data.len() {
                return Err(EncodeError::EncodeFailed(
                    "MFT returned an invalid H.264 packet".into(),
                ));
            }
        }
        return Ok(data.to_vec());
    }

    let avcc = annex_b_to_avcc(data);
    if is_valid_avcc(&avcc) {
        Ok(avcc)
    } else {
        Err(EncodeError::EncodeFailed(
            "MFT returned an invalid H.264 packet".into(),
        ))
    }
}

fn is_valid_avcc(data: &[u8]) -> bool {
    let mut offset = 0;
    let mut nal_count = 0;

    while offset + 4 <= data.len() {
        let nal_len = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if nal_len == 0 || offset + nal_len > data.len() {
            return false;
        }
        offset += nal_len;
        nal_count += 1;
    }

    nal_count > 0 && offset == data.len()
}

/// H.264 IDR slices (NAL type 5) are independently decodable keyframes.
/// Some Media Foundation encoders omit `MFSampleExtension_CleanPoint` on their
/// output sample, so relying on that metadata alone loses valid keyframes.
fn avcc_contains_idr(data: &[u8]) -> bool {
    let mut offset = 0;

    while offset + 4 <= data.len() {
        let nal_len = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        let nal_start = offset + 4;
        let nal_end = nal_start + nal_len;
        if nal_len == 0 || nal_end > data.len() {
            return false;
        }
        if data[nal_start] & 0x1F == 5 {
            return true;
        }
        offset = nal_end;
    }

    false
}

// ---------------------------------------------------------------------------
// SPS/PPS access
// ---------------------------------------------------------------------------

impl MfH264Encoder {
    /// Return the cached SPS NAL unit (AVCC format), if available.
    pub fn sps(&self) -> &[u8] {
        &self.sps
    }

    /// Return the cached PPS NAL unit (AVCC format), if available.
    pub fn pps(&self) -> &[u8] {
        &self.pps
    }

    /// Whether SPS/PPS have been captured from the encoder output.
    pub fn sps_pps_ready(&self) -> bool {
        self.sps_pps_ready
    }
}

/// Scan AVCC data for SPS (NAL type 7) and PPS (NAL type 8), preserving their
/// AVCC length prefixes for use by the clip-preparation path.
fn capture_sps_pps_from_avcc(data: &[u8], sps_out: &mut Vec<u8>, pps_out: &mut Vec<u8>) {
    let mut offset = 0;
    while offset + 4 <= data.len() {
        let nal_len = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        let nal_start = offset + 4;
        let nal_end = nal_start + nal_len;
        if nal_len == 0 || nal_end > data.len() {
            break;
        }

        match data[nal_start] & 0x1F {
            7 => *sps_out = data[offset..nal_end].to_vec(),
            8 => *pps_out = data[offset..nal_end].to_vec(),
            _ => {}
        }
        offset = nal_end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn avcc(nals: &[&[u8]]) -> Vec<u8> {
        let mut data = Vec::new();
        for nal in nals {
            data.extend_from_slice(&(nal.len() as u32).to_be_bytes());
            data.extend_from_slice(nal);
        }
        data
    }

    #[test]
    fn packs_media_foundation_size_and_rate_in_api_order() {
        assert_eq!(pack_mf_attribute_pair(1920, 1080), 0x0000_0780_0000_0438);
        assert_eq!(pack_mf_attribute_pair(60, 1), 0x0000_003C_0000_0001);
    }

    #[test]
    fn preserves_avcc_samples_and_captures_parameter_sets() {
        let sps = [0x67, 0x42, 0x00, 0x1E];
        let pps = [0x68, 0xCE, 0x06, 0xE2];
        let idr = [0x65, 0x88, 0x84];
        let packet = avcc(&[&sps, &pps, &idr]);

        let normalized = h264_to_avcc(&packet).unwrap();
        let mut found_sps = Vec::new();
        let mut found_pps = Vec::new();
        capture_sps_pps_from_avcc(&normalized, &mut found_sps, &mut found_pps);

        assert_eq!(normalized, packet);
        assert_eq!(found_sps, avcc(&[&sps]));
        assert_eq!(found_pps, avcc(&[&pps]));
    }

    #[test]
    fn converts_annex_b_samples_and_captures_parameter_sets() {
        let sps = [0x67, 0x42, 0x00, 0x1E];
        let pps = [0x68, 0xCE, 0x06, 0xE2];
        let idr = [0x65, 0x88, 0x84];
        let mut annex_b = Vec::new();
        for nal in [&sps[..], &pps[..], &idr[..]] {
            annex_b.extend_from_slice(&[0, 0, 0, 1]);
            annex_b.extend_from_slice(nal);
        }

        let normalized = h264_to_avcc(&annex_b).unwrap();
        let mut found_sps = Vec::new();
        let mut found_pps = Vec::new();
        capture_sps_pps_from_avcc(&normalized, &mut found_sps, &mut found_pps);

        assert_eq!(normalized, avcc(&[&sps, &pps, &idr]));
        assert_eq!(found_sps, avcc(&[&sps]));
        assert_eq!(found_pps, avcc(&[&pps]));
    }

    #[test]
    fn rejects_malformed_h264_packets() {
        let error = h264_to_avcc(&[0, 0, 0, 8, 0x65]).unwrap_err();

        assert!(error.to_string().contains("invalid H.264 packet"));
    }

    #[test]
    fn detects_idr_keyframes_without_clean_point_metadata() {
        let p_slice = [0x41, 0x9A, 0x22];
        let idr = [0x65, 0x88, 0x84];

        assert!(!avcc_contains_idr(&avcc(&[&p_slice])));
        assert!(avcc_contains_idr(&avcc(&[&p_slice, &idr])));
    }
}

// ---------------------------------------------------------------------------
// Fallback: SPS/PPS from output media type
// ---------------------------------------------------------------------------

/// Extract SPS/PPS from the MFT's output media type via
/// `MF_MT_MPEG_SEQUENCE_HEADER`. This is a fallback for MFT implementations
/// that don't include SPS/PPS in the encoded bitstream.
///
/// The blob is in AVCC extradata format:
///   [1B version] [1B profile] [1B compat] [1B level]
///   [1B: 0xFC | (nal_length_size-1)] [1B: 0xE0 | num_sps]
///   for each SPS: [2B length][SPS NAL]
///   [1B num_pps]
///   for each PPS: [2B length][PPS NAL]
fn capture_sps_pps_from_media_type(
    transform: &IMFTransform,
    sps_out: &mut Vec<u8>,
    pps_out: &mut Vec<u8>,
) -> Result<(), EncodeError> {
    unsafe {
        let current_type = transform
            .GetOutputCurrentType(0)
            .map_err(|e| EncodeError::EncodeFailed(format!("GetOutputCurrentType: {e}")))?;

        let mut blob_size = current_type
            .GetBlobSize(&MF_MT_MPEG_SEQUENCE_HEADER)
            .map_err(|e| EncodeError::EncodeFailed(format!("GetBlobSize: {e}")))?;

        if blob_size == 0 {
            return Err(EncodeError::EncodeFailed("Empty sequence header".into()));
        }

        let mut blob = vec![0u8; blob_size as usize];
        let p_blob_size: *mut u32 = &mut blob_size;
        current_type
            .GetBlob(&MF_MT_MPEG_SEQUENCE_HEADER, &mut blob, Some(p_blob_size))
            .map_err(|e| EncodeError::EncodeFailed(format!("GetBlob: {e}")))?;

        if blob_size as usize > blob.len() {
            blob.resize(blob_size as usize, 0);
            current_type
                .GetBlob(&MF_MT_MPEG_SEQUENCE_HEADER, &mut blob, Some(p_blob_size))
                .map_err(|e| EncodeError::EncodeFailed(format!("GetBlob retry: {e}")))?;
        }

        drop(current_type);

        // Parse AVCC extradata
        if blob.len() < 6 {
            return Err(EncodeError::EncodeFailed(
                "Sequence header too short".into(),
            ));
        }

        let num_sps = (blob[5] & 0x1F) as usize;
        let mut offset = 6usize;

        for _ in 0..num_sps {
            if offset + 2 > blob.len() {
                break;
            }
            let sps_len = u16::from_be_bytes([blob[offset], blob[offset + 1]]) as usize;
            offset += 2;
            if offset + sps_len > blob.len() {
                break;
            }
            if sps_out.is_empty() {
                let mut avcc = Vec::with_capacity(4 + sps_len);
                avcc.extend_from_slice(&(sps_len as u32).to_be_bytes());
                avcc.extend_from_slice(&blob[offset..offset + sps_len]);
                *sps_out = avcc;
            }
            offset += sps_len;
        }

        if offset >= blob.len() {
            return Ok(());
        }

        let num_pps = blob[offset] as usize;
        offset += 1;

        for _ in 0..num_pps {
            if offset + 2 > blob.len() {
                break;
            }
            let pps_len = u16::from_be_bytes([blob[offset], blob[offset + 1]]) as usize;
            offset += 2;
            if offset + pps_len > blob.len() {
                break;
            }
            if pps_out.is_empty() {
                let mut avcc = Vec::with_capacity(4 + pps_len);
                avcc.extend_from_slice(&(pps_len as u32).to_be_bytes());
                avcc.extend_from_slice(&blob[offset..offset + pps_len]);
                *pps_out = avcc;
            }
            offset += pps_len;
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Drop: clean up MF resources
// ---------------------------------------------------------------------------

impl Drop for MfH264Encoder {
    fn drop(&mut self) {
        unsafe {
            self.transform
                .ProcessMessage(MFT_MESSAGE_NOTIFY_END_STREAMING, 0)
                .ok();
            self.transform
                .ProcessMessage(MFT_MESSAGE_COMMAND_FLUSH, 0)
                .ok();
        }
    }
}
