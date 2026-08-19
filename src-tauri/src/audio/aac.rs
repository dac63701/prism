//! Media Foundation AAC encoder (Windows).
//!
//! Converts captured float32 stereo PCM into raw AAC frames (MF_MT_AAC_PAYLOAD_TYPE
//! = 0, i.e. no ADTS header) suitable for direct MP4 muxing via the `mp4` crate.
//! The encoder processes 1024 samples per AAC frame at 48 kHz (~21.33 ms).

use std::sync::OnceLock;

use windows::core::GUID;
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::*;

use crate::encoder::codecs::{AacFrame, AudioClip};
use crate::encoder::EncodeError;

// ---------------------------------------------------------------------------
// One-shot MF startup (independent of the H.264 encoder's static)
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
// Media type helpers
// ---------------------------------------------------------------------------

/// Compare a media type's `MF_MT_SUBTYPE` against a GUID.
unsafe fn matches_type_subtype(mt: &IMFMediaType, guid: &GUID) -> bool {
    matches!(mt.GetGUID(&MF_MT_SUBTYPE), Ok(s) if &s == guid)
}

/// Find an advertised input media type matching `pred`. The AAC encoder's
/// advertised input types reflect the configured output type, so this must be
/// called AFTER `SetOutputType`. Falls back to the first advertised type.
unsafe fn find_input_type(
    transform: &IMFTransform,
    pred: impl Fn(&IMFMediaType) -> bool,
) -> Result<IMFMediaType, EncodeError> {
    let mut fallback: Option<IMFMediaType> = None;
    for i in 0..64 {
        let Ok(mt) = transform.GetInputAvailableType(0, i) else {
            break;
        };
        if pred(&mt) {
            return Ok(mt);
        }
        fallback.get_or_insert(mt);
    }
    fallback
        .ok_or_else(|| EncodeError::InitFailed("AAC encoder advertised no input media type".into()))
}

/// Find an advertised output media type matching `pred`. The AAC encoder
/// advertises a matrix of sample-rate × channel combinations, so iterate until
/// the MFT reports no more types. Falls back to the first advertised type when
/// nothing matches `pred`.
unsafe fn find_output_type(
    transform: &IMFTransform,
    pred: impl Fn(&IMFMediaType) -> bool,
) -> Result<IMFMediaType, EncodeError> {
    let mut fallback: Option<IMFMediaType> = None;
    for i in 0..64 {
        let Ok(mt) = transform.GetOutputAvailableType(0, i) else {
            break;
        };
        if pred(&mt) {
            return Ok(mt);
        }
        fallback.get_or_insert(mt);
    }
    fallback.ok_or_else(|| {
        EncodeError::InitFailed("AAC encoder advertised no output media type".into())
    })
}

// ---------------------------------------------------------------------------
// AAC encoder
// ---------------------------------------------------------------------------

/// Wraps the Media Foundation AAC Encoder MFT for a single clip encode pass.
pub struct AacEncoder {
    transform: IMFTransform,
    sample_rate: u32,
    channels: u16,
    /// Reusable input sample + buffer, avoiding per-chunk MF allocations.
    input_sample: Option<IMFSample>,
    input_buffer: Option<IMFMediaBuffer>,
    /// Cached output stream info (whether the MFT provides its own samples).
    provides_samples: bool,
}

// SAFETY: Same argument as `MfH264Encoder` — the COM transform is used from a
// single thread at a time via `&mut self`; COM is initialized by MFStartup.
unsafe impl Send for AacEncoder {}

/// Samples per AAC frame at 48 kHz (the encoder's native frame size).
const AAC_FRAME_SAMPLES: u32 = 1024;

impl AacEncoder {
    /// Create an AAC encoder for the given sample rate / channel layout.
    pub fn new(sample_rate: u32, channels: u16, bitrate_kbps: u32) -> Result<Self, EncodeError> {
        ensure_mf()?;

        unsafe {
            let transform: IMFTransform =
                CoCreateInstance(&AACMFTEncoder, None, CLSCTX_INPROC_SERVER).map_err(|e| {
                    EncodeError::InitFailed(format!("CoCreateInstance AAC encoder: {e}"))
                })?;

            // ------ Output type: raw AAC ------
            // The MF AAC encoder advertises a matrix of output types (sample
            // rate × channels). Pick the AAC type matching our capture format,
            // then override bitrate + raw payload type. The output must be set
            // FIRST — the input type the encoder advertises depends on it.
            let output_type: IMFMediaType = find_output_type(&transform, |mt| {
                matches_type_subtype(mt, &MFAudioFormat_AAC)
                    && mt.GetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND) == Ok(sample_rate)
                    && mt.GetUINT32(&MF_MT_AUDIO_NUM_CHANNELS) == Ok(channels as u32)
            })?;
            output_type
                .SetUINT32(&MF_MT_AVG_BITRATE, bitrate_kbps.saturating_mul(1_000))
                .ok();
            output_type
                .SetUINT32(&MF_MT_AAC_PAYLOAD_TYPE, 0) // raw, no ADTS
                .ok();
            output_type
                .SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, sample_rate)
                .ok();
            output_type
                .SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, channels as u32)
                .ok();

            transform
                .SetOutputType(0, &output_type, 0)
                .map_err(|e| EncodeError::InitFailed(format!("SetOutputType: {e}")))?;

            // ------ Input type: int16 PCM ------
            // After the output is configured, the encoder advertises matching
            // input types. Use the PCM (int16) one at our sample rate / channel
            // count verbatim — overriding attributes makes the type inconsistent
            // and SetInputType rejects it.
            let input_type: IMFMediaType = find_input_type(&transform, |mt| {
                matches_type_subtype(mt, &MFAudioFormat_PCM)
                    && mt.GetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND) == Ok(sample_rate)
                    && mt.GetUINT32(&MF_MT_AUDIO_NUM_CHANNELS) == Ok(channels as u32)
            })?;

            transform
                .SetInputType(0, &input_type, 0)
                .map_err(|e| EncodeError::InitFailed(format!("SetInputType: {e}")))?;

            let info = transform
                .GetOutputStreamInfo(0)
                .map_err(|e| EncodeError::InitFailed(format!("GetOutputStreamInfo: {e}")))?;
            let provider_flags = (MFT_OUTPUT_STREAM_PROVIDES_SAMPLES.0
                | MFT_OUTPUT_STREAM_CAN_PROVIDE_SAMPLES.0) as u32;

            transform
                .ProcessMessage(MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, 0)
                .map_err(|e| EncodeError::InitFailed(format!("Begin streaming: {e}")))?;
            transform
                .ProcessMessage(MFT_MESSAGE_NOTIFY_START_OF_STREAM, 0)
                .map_err(|e| EncodeError::InitFailed(format!("Start stream: {e}")))?;

            Ok(Self {
                transform,
                sample_rate,
                channels,
                input_sample: None,
                input_buffer: None,
                provides_samples: info.dwFlags & provider_flags != 0,
            })
        }
    }

    /// Encode an entire PCM clip (int16 interleaved, `channels` per frame)
    /// into sequential AAC frames. The input should contain a whole number of
    /// 1024-sample frames; a partial tail is silently dropped.
    pub fn encode_pcm(&mut self, pcm: &[u8]) -> Result<Vec<AacFrame>, EncodeError> {
        let bytes_per_frame = (self.channels as usize) * 2;
        if !pcm.len().is_multiple_of(bytes_per_frame) {
            return Err(EncodeError::EncodeFailed(format!(
                "PCM length {} is not a multiple of {bytes_per_frame}",
                pcm.len()
            )));
        }

        let samples_per_channel = pcm.len() / bytes_per_frame;
        let frame_bytes = (AAC_FRAME_SAMPLES as usize) * bytes_per_frame;
        let frames = samples_per_channel / AAC_FRAME_SAMPLES as usize;

        let mut out = Vec::new();
        for i in 0..frames {
            let chunk = &pcm[i * frame_bytes..(i + 1) * frame_bytes];
            // SAFETY: `encode_chunk` is internal and serialized on `&mut self`.
            let encoded = unsafe { self.encode_chunk(chunk, i as i64) }?;
            out.extend(encoded);
        }
        Ok(out)
    }

    /// Feed one 1024-sample chunk and collect whatever the MFT outputs.
    unsafe fn encode_chunk(
        &mut self,
        chunk: &[u8],
        chunk_index: i64,
    ) -> Result<Vec<AacFrame>, EncodeError> {
        let (sample, buffer) = match (self.input_sample.as_ref(), self.input_buffer.as_ref()) {
            (Some(s), Some(b)) => (s.clone(), b.clone()),
            _ => {
                let buffer: IMFMediaBuffer = MFCreateMemoryBuffer(chunk.len() as u32)
                    .map_err(|e| EncodeError::EncodeFailed(format!("CreateMemoryBuffer: {e}")))?;
                let sample: IMFSample = MFCreateSample()
                    .map_err(|e| EncodeError::EncodeFailed(format!("CreateSample: {e}")))?;
                sample
                    .AddBuffer(&buffer)
                    .map_err(|e| EncodeError::EncodeFailed(format!("AddBuffer: {e}")))?;
                self.input_sample = Some(sample.clone());
                self.input_buffer = Some(buffer.clone());
                (sample, buffer)
            }
        };

        let mut ptr: *mut u8 = std::ptr::null_mut();
        let mut max_len: u32 = 0;
        let mut cur_len: u32 = 0;
        buffer
            .Lock(&mut ptr, Some(&mut max_len), Some(&mut cur_len))
            .map_err(|e| EncodeError::EncodeFailed(format!("Lock buffer: {e}")))?;
        std::ptr::copy_nonoverlapping(chunk.as_ptr(), ptr, chunk.len());
        buffer
            .SetCurrentLength(chunk.len() as u32)
            .map_err(|e| EncodeError::EncodeFailed(format!("SetCurrentLength: {e}")))?;
        buffer
            .Unlock()
            .map_err(|e| EncodeError::EncodeFailed(format!("Unlock buffer: {e}")))?;

        // 1024 samples at `sample_rate`, expressed in 100ns units.
        let duration_100ns = (AAC_FRAME_SAMPLES as i64 * 10_000_000) / self.sample_rate as i64;
        let timestamp_100ns = chunk_index * duration_100ns;
        sample
            .SetSampleTime(timestamp_100ns)
            .map_err(|e| EncodeError::EncodeFailed(format!("SetSampleTime: {e}")))?;
        sample
            .SetSampleDuration(duration_100ns)
            .map_err(|e| EncodeError::EncodeFailed(format!("SetSampleDuration: {e}")))?;

        self.transform
            .ProcessInput(0, &sample, 0)
            .map_err(|e| EncodeError::EncodeFailed(format!("ProcessInput: {e}")))?;

        let mut packets = Vec::new();
        loop {
            let mut output = self.create_output_buffer()?;
            let mut status: u32 = 0;
            let result =
                self.transform
                    .ProcessOutput(0, std::slice::from_mut(&mut output), &mut status);

            if result.is_ok() {
                let packet_result = (|| -> Result<Option<AacFrame>, EncodeError> {
                    let Some(ref out_sample) = *output.pSample else {
                        return Ok(None);
                    };
                    let data = collect_sample_bytes(out_sample)?;
                    let start_time = match out_sample.GetSampleTime() {
                        Ok(t) => t.max(0) as u64,
                        Err(_) => chunk_index as u64 * AAC_FRAME_SAMPLES as u64,
                    };
                    Ok(Some(AacFrame {
                        start_time,
                        duration: AAC_FRAME_SAMPLES,
                        data,
                    }))
                })();
                release_output_buffer(&mut output);
                if let Some(packet) = packet_result? {
                    packets.push(packet);
                }
            } else if let Err(err) = &result {
                if err.code() == MF_E_TRANSFORM_NEED_MORE_INPUT {
                    release_output_buffer(&mut output);
                    break;
                }
                release_output_buffer(&mut output);
                return Err(EncodeError::EncodeFailed(format!("ProcessOutput: {err}")));
            }
        }

        Ok(packets)
    }

    /// Create the output descriptor required by this MFT.
    unsafe fn create_output_buffer(&self) -> Result<MFT_OUTPUT_DATA_BUFFER, EncodeError> {
        if self.provides_samples {
            return Ok(MFT_OUTPUT_DATA_BUFFER {
                dwStreamID: 0,
                ..Default::default()
            });
        }

        let info = self
            .transform
            .GetOutputStreamInfo(0)
            .map_err(|e| EncodeError::EncodeFailed(format!("GetOutputStreamInfo: {e}")))?;
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

    /// Drain remaining buffered frames after the last chunk.
    /// Must be called to flush the encoder's internal latency.
    pub fn drain(&mut self) -> Result<Vec<AacFrame>, EncodeError> {
        // SAFETY: serialized on `&mut self`.
        unsafe {
            self.transform
                .ProcessMessage(MFT_MESSAGE_COMMAND_DRAIN, 0)
                .map_err(|e| EncodeError::EncodeFailed(format!("Drain: {e}")))?;
        }

        let mut packets = Vec::new();
        unsafe {
            loop {
                let mut output = self.create_output_buffer()?;
                let mut status: u32 = 0;
                let result =
                    self.transform
                        .ProcessOutput(0, std::slice::from_mut(&mut output), &mut status);

                if result.is_ok() {
                    let packet_result = (|| -> Result<Option<AacFrame>, EncodeError> {
                        let Some(ref out_sample) = *output.pSample else {
                            return Ok(None);
                        };
                        let data = collect_sample_bytes(out_sample)?;
                        let start_time = match out_sample.GetSampleTime() {
                            Ok(t) => t.max(0) as u64,
                            Err(_) => 0,
                        };
                        Ok(Some(AacFrame {
                            start_time,
                            duration: AAC_FRAME_SAMPLES,
                            data,
                        }))
                    })();
                    release_output_buffer(&mut output);
                    if let Some(packet) = packet_result? {
                        packets.push(packet);
                    }
                } else if let Err(err) = &result {
                    if err.code() == MF_E_TRANSFORM_NEED_MORE_INPUT {
                        release_output_buffer(&mut output);
                        break;
                    }
                    release_output_buffer(&mut output);
                    return Err(EncodeError::EncodeFailed(format!(
                        "Drain ProcessOutput: {err}"
                    )));
                }
            }
        }

        Ok(packets)
    }

    #[allow(dead_code)]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    #[allow(dead_code)]
    pub fn channels(&self) -> u16 {
        self.channels
    }
}

/// The generated bindings use `ManuallyDrop` for COM pointers in this FFI
/// struct, so release them explicitly after every `ProcessOutput` call.
unsafe fn release_output_buffer(output: &mut MFT_OUTPUT_DATA_BUFFER) {
    std::mem::ManuallyDrop::drop(&mut output.pSample);
    std::mem::ManuallyDrop::drop(&mut output.pEvents);
}

/// Collect bytes from an output sample.
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
// High-level helper
// ---------------------------------------------------------------------------

/// Encode a complete PCM clip (float32 interleaved) into an [`AudioClip`].
pub fn encode_clip_audio(
    pcm: &[u8],
    sample_rate: u32,
    channels: u16,
    bitrate_kbps: u32,
) -> Result<AudioClip, EncodeError> {
    let mut encoder = AacEncoder::new(sample_rate, channels, bitrate_kbps)?;
    let mut frames = encoder.encode_pcm(pcm)?;
    frames.extend(encoder.drain()?);
    // AAC frames come out in 100ns sample-time units; the muxer expects audio
    // timescale (sample_rate) units. Convert 100ns → sample index.
    let timescale = sample_rate as u64;
    for frame in frames.iter_mut() {
        frame.start_time = (frame.start_time * timescale) / 10_000_000;
    }
    Ok(AudioClip {
        sample_rate,
        channels: channels as u8,
        bitrate_kbps,
        frames,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aac_frame_size_and_channel_count() {
        let encoder = AacEncoder::new(48_000, 2, 128).expect("encoder init");
        assert_eq!(encoder.sample_rate(), 48_000);
        assert_eq!(encoder.channels(), 2);
    }

    #[test]
    fn encodes_a_second_of_pcm_into_aac_frames() {
        // 1 second of 1 kHz sine, int16 stereo interleaved.
        let sample_rate = 48_000u32;
        let channels = 2u16;
        let mut pcm = Vec::with_capacity(sample_rate as usize * channels as usize * 2);
        for i in 0..sample_rate as usize {
            let sample = ((i as f32 * 2.0 * std::f32::consts::PI * 1000.0 / sample_rate as f32)
                .sin()
                * 0.25
                * i16::MAX as f32) as i16;
            for _ in 0..channels {
                pcm.extend_from_slice(&sample.to_le_bytes());
            }
        }

        let clip = encode_clip_audio(&pcm, sample_rate, channels, 128).expect("encode");
        // 48 kHz / 1024 samples per frame ≈ 46.9 frames per second.
        assert!(
            clip.frames.len() >= 44 && clip.frames.len() <= 48,
            "expected ~47 AAC frames, got {}",
            clip.frames.len()
        );
        for frame in &clip.frames {
            assert_eq!(frame.duration, AAC_FRAME_SAMPLES);
            assert!(!frame.data.is_empty(), "AAC frame should carry payload");
            // Raw AAC payload for a 128 kbps stereo LC frame is ~200–900 bytes.
            assert!(
                frame.data.len() >= 100 && frame.data.len() <= 1200,
                "unexpected AAC frame size {}",
                frame.data.len()
            );
        }
        assert_eq!(clip.sample_rate, sample_rate);
        assert_eq!(clip.channels, channels as u8);
    }
}
