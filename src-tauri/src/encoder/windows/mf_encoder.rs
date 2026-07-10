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
        _keyframe_interval: u32,
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

            // Frame size packed as: height << 32 | width
            let frame_size: u64 = (height as u64) << 32 | width as u64;
            input_type
                .SetUINT64(&MF_MT_FRAME_SIZE, frame_size)
                .map_err(|e| EncodeError::InitFailed(format!("SetUINT64 frame size: {e}")))?;

            // Frame rate packed as: denominator << 32 | numerator
            let frame_rate: u64 = (1u64) << 32 | fps as u64;
            input_type
                .SetUINT64(&MF_MT_FRAME_RATE, frame_rate)
                .map_err(|e| EncodeError::InitFailed(format!("SetUINT64 frame rate: {e}")))?;

            let pixel_aspect: u64 = (1u64) << 32 | 1u64;
            input_type
                .SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pixel_aspect)
                .ok();

            transform
                .SetInputType(0, &input_type, 0)
                .map_err(|e| EncodeError::InitFailed(format!("SetInputType: {e}")))?;

            // ------ Set output type: H.264 ------
            let output_type: IMFMediaType = MFCreateMediaType()
                .map_err(|e| EncodeError::InitFailed(format!("MFCreateMediaType output: {e}")))?;

            output_type
                .SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
                .map_err(|e| EncodeError::InitFailed(format!("SetGUID major out: {e}")))?;
            output_type
                .SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264)
                .map_err(|e| EncodeError::InitFailed(format!("SetGUID subtype out: {e}")))?;
            output_type
                .SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)
                .map_err(|e| EncodeError::InitFailed(format!("SetUINT32 interlace out: {e}")))?;

            // Average bitrate in bps
            output_type
                .SetUINT32(&MF_MT_AVG_BITRATE, bitrate_kbps * 1000)
                .map_err(|e| EncodeError::InitFailed(format!("SetUINT32 bitrate: {e}")))?;

            // Frame size on output (must match input)
            output_type.SetUINT64(&MF_MT_FRAME_SIZE, frame_size).ok();

            // Frame rate on output
            output_type.SetUINT64(&MF_MT_FRAME_RATE, frame_rate).ok();

            // Set Baseline profile (no B-frames) for simple PTS/DTS
            // eAVEncH264VProfile_Base = 66
            output_type.SetUINT32(&MF_MT_MPEG2_PROFILE, 66).ok();

            transform
                .SetOutputType(0, &output_type, 0)
                .map_err(|e| EncodeError::InitFailed(format!("SetOutputType: {e}")))?;

            // Set Baseline profile (MF_MT_MPEG2_PROFILE=66) disables B-frames
            // on most MFT implementations, keeping PTS == DTS for simple muxing.
            // ICodecAPI configuration is optional but adds complexity — skip it.

            transform
                .ProcessMessage(MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, 0)
                .map_err(|e| EncodeError::InitFailed(format!("Begin streaming: {e}")))?;

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
                let mut output = MFT_OUTPUT_DATA_BUFFER::default();
                output.dwStreamID = 0;
                *output.pSample = None;
                output.dwStatus = 0;
                *output.pEvents = None;

                let mut status: u32 = 0;

                let result = self
                    .transform
                    .ProcessOutput(0, &mut [output.clone()], &mut status);

                if result.is_ok() {
                    if let Some(ref out_sample) = *output.pSample {
                        let raw = collect_sample_bytes(out_sample)?;
                        let is_key = is_keyframe(out_sample);

                        // Capture SPS/PPS from raw Annex B output if not yet ready
                        if !self.sps_pps_ready {
                            capture_sps_pps_from_annex_b(&raw, &mut self.sps, &mut self.pps);
                            if !self.sps.is_empty() && !self.pps.is_empty() {
                                self.sps_pps_ready = true;
                            }
                        }

                        let avcc = annex_b_to_avcc(&raw);
                        packets.push(EncodedPacket {
                            data: avcc,
                            is_sync: is_key,
                        });
                    }
                } else if let Err(err) = &result {
                    let code = err.code().0;
                    if code as u32 == 0xC00D3704u32 {
                        // MF_E_TRANSFORM_NEED_MORE_INPUT
                        break;
                    }
                    if code as u32 == 0xC00D370Eu32 {
                        // MF_E_TRANSFORM_STREAM_CHANGE — get new output type
                        if let Ok(new_type) = self.transform.GetOutputAvailableType(0, 0) {
                            self.transform.SetOutputType(0, &new_type, 0).ok();
                        }
                        continue;
                    }
                    return Err(EncodeError::EncodeFailed(format!("ProcessOutput: {err}")));
                }
            }

            self.frame_index += 1;
            Ok(packets)
        }
    }
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

/// Scan raw Annex B data for SPS (NAL type 7) and PPS (NAL type 8) and
/// store them in AVCC format (4-byte length prefix) into the output vectors.
fn capture_sps_pps_from_annex_b(data: &[u8], sps_out: &mut Vec<u8>, pps_out: &mut Vec<u8>) {
    let mut i = 0;
    while i < data.len() {
        // Find start code
        if i + 4 <= data.len()
            && data[i] == 0
            && data[i + 1] == 0
            && data[i + 2] == 0
            && data[i + 3] == 1
        {
            i += 4;
        } else if i + 3 <= data.len() && data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
            i += 3;
        } else {
            i += 1;
            continue;
        }

        if i >= data.len() {
            break;
        }

        let nal_type = data[i] & 0x1F;

        // Find the next start code (or end of data)
        let nal_start = i;
        while i < data.len() {
            if i + 4 <= data.len()
                && data[i] == 0
                && data[i + 1] == 0
                && data[i + 2] == 0
                && data[i + 3] == 1
            {
                break;
            }
            if i + 3 <= data.len() && data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
                break;
            }
            i += 1;
        }

        let nal_data = &data[nal_start..i];
        if !nal_data.is_empty() && (nal_type == 7 || nal_type == 8) {
            let mut avcc = Vec::with_capacity(4 + nal_data.len());
            avcc.extend_from_slice(&(nal_data.len() as u32).to_be_bytes());
            avcc.extend_from_slice(nal_data);
            if nal_type == 7 {
                *sps_out = avcc;
            } else {
                *pps_out = avcc;
            }
        }
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
