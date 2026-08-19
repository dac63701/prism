//! Media Foundation H.264 hardware encoder wrapper.
//!
//! Wraps the H.264 Video Encoder MFT (NVENC / AMF / QuickSync depending on GPU)
//! to produce compressed H.264 packets from NV12 input frames.
//!
//! The encoder outputs H.264 Annex B byte-stream which is converted to AVCC
//! (4-byte length-prefix format) suitable for direct MP4 muxing.

use std::collections::VecDeque;
use std::sync::OnceLock;

use windows::core::Interface;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
    D3D11_BIND_SHADER_RESOURCE, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
    D3D11_CREATE_DEVICE_VIDEO_SUPPORT, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_SDK_VERSION,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_NV12, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::IDXGIAdapter1;
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Variant::{VARIANT, VT_UI4};

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

/// Exact byte size of a tightly-packed NV12 frame: Y plane (`w*h`) followed by
/// interleaved UV (`ceil(w/2)*ceil(h/2)*2`). Equals `w*h*3/2` for even
/// dimensions; larger for odd ones. Must match `bgra_to_nv12` and the capture
/// staging packers so the encoder's strict size guard accepts real frames.
fn nv12_packed_size(width: u32, height: u32) -> usize {
    let w = width as usize;
    let h = height as usize;
    w * h + w.div_ceil(2) * h.div_ceil(2) * 2
}

// ---------------------------------------------------------------------------
// Encoded packet
// ---------------------------------------------------------------------------

/// A single compressed H.264 NAL unit or combined packet in AVCC format.
pub struct EncodedPacket {
    pub data: Vec<u8>,
    pub is_sync: bool,
    /// Capture timestamp of the source frame. For sync encoders this is the
    /// timestamp of the exact input frame; for async encoders it is attributed
    /// from the FIFO of submitted inputs (output lags input by the encoder's
    /// internal latency).
    pub timestamp: std::time::Instant,
}

/// A hardware H.264 encoder candidate found by [`enumerate_h264_encoders`].
struct MfHardwareCandidate {
    transform: IMFTransform,
    name: String,
    is_async: bool,
    /// `MFT_ENUM_ADAPTER_LUID` of the GPU this MFT is bound to, when the MFT
    /// advertises one. Hardware encoders reject an `IMFDXGIDeviceManager`
    /// whose D3D11 device sits on a different adapter, so on multi-GPU
    /// machines (e.g. AMD iGPU + NVIDIA dGPU) the matching adapter must be
    /// used to create the device.
    adapter_luid: Option<u64>,
}

/// Enumerate hardware H.264 encoder MFTs (NVENC / AMF / QuickSync).
///
/// Returns `Ok(empty)` when no hardware encoder is registered (e.g. a GPU with
/// no Media Foundation encoder or `PRISM_FORCE_MS_H264=1`), so the caller can
/// fall back to the Microsoft software encoder explicitly. Transforms are only
/// usable once an `IMFDXGIDeviceManager` is attached (see
/// [`MfH264Encoder::from_transform`]); without one, most hardware MFTs reject
/// CPU-supplied NV12 media types.
///
/// Both synchronous and asynchronous hardware MFTs are returned. Async MFTs
/// (AMD/NVIDIA) are driven through the event-based processing model by
/// [`MfH264Encoder`]; sync MFTs (e.g. Intel QuickSync) use the direct model.
fn enumerate_h264_encoders() -> Result<Vec<MfHardwareCandidate>, EncodeError> {
    if std::env::var("PRISM_FORCE_MS_H264").as_deref() == Ok("1") {
        eprintln!("[h264] PRISM_FORCE_MS_H264=1, skipping hardware encoders");
        return Ok(Vec::new());
    }

    // Enumerate hardware H.264 video encoders. Filter by input (NV12) and
    // output (H.264) media types so only compatible encoders are considered,
    // and sort so the best match comes first.
    let input_type = MFT_REGISTER_TYPE_INFO {
        guidMajorType: MFMediaType_Video,
        guidSubtype: MFVideoFormat_NV12,
    };
    let output_type = MFT_REGISTER_TYPE_INFO {
        guidMajorType: MFMediaType_Video,
        guidSubtype: MFVideoFormat_H264,
    };

    let mut activates: *mut Option<IMFActivate> = std::ptr::null_mut();
    let mut count: u32 = 0;
    let result = unsafe {
        MFTEnumEx(
            MFT_CATEGORY_VIDEO_ENCODER,
            MFT_ENUM_FLAG_HARDWARE | MFT_ENUM_FLAG_SORTANDFILTER,
            Some(&input_type),
            Some(&output_type),
            &mut activates,
            &mut count,
        )
    };

    if result.is_ok() && count > 0 && !activates.is_null() {
        let mut candidates = Vec::new();
        // SAFETY: `MFTEnumEx` allocated an array of `count` activate pointers
        // that we own and must free with `CoTaskMemFree`.
        let activates_slice = unsafe { std::slice::from_raw_parts(activates, count as usize) };
        for (i, activate) in activates_slice.iter().enumerate() {
            if let Some(activate) = activate {
                let name = mft_friendly_name(activate);
                let is_async = unsafe { activate.GetUINT32(&MF_TRANSFORM_ASYNC) }.unwrap_or(0) != 0;
                let adapter_luid = unsafe { activate.GetUINT64(&MFT_ENUM_ADAPTER_LUID) }.ok();
                if adapter_luid.is_some() {
                    eprintln!(
                        "[h264] hardware encoder \"{name}\" adapter LUID=0x{:x}",
                        adapter_luid.unwrap()
                    );
                }
                match unsafe { activate.ActivateObject::<IMFTransform>() } {
                    Ok(transform) => {
                        eprintln!(
                            "[h264] hardware encoder MFT #{i} (\"{name}\", async={is_async})"
                        );
                        candidates.push(MfHardwareCandidate {
                            transform,
                            name,
                            is_async,
                            adapter_luid,
                        });
                    }
                    Err(e) => {
                        eprintln!("[h264] hardware encoder #{i} failed to activate: {e}");
                    }
                }
            }
        }
        // Free the activate-pointer array; the selected transforms are already AddRef'd.
        unsafe {
            windows::Win32::System::Com::CoTaskMemFree(Some(activates as *const _));
        }
        return Ok(candidates);
    } else if result.is_err() {
        eprintln!("[h264] MFTEnumEx failed");
    }

    Ok(Vec::new())
}
/// Create a D3D11 device (BGRA support, per MSDN) and an `IMFDXGIDeviceManager`
/// associated with it. Hardware H.264 MFTs require this manager
/// (`MFT_MESSAGE_SET_D3D_MANAGER`) before they accept CPU-supplied NV12 input;
/// the Microsoft software encoder ignores it entirely. The device context is
/// returned too — async hardware MFTs need it to Map/Unmap the D3D11 NV12
/// textures that carry input frames.
///
/// When `adapter_luid` is `Some`, the device is created on the DXGI adapter
/// matching that LUID. Hardware MFTs reject an `IMFDXGIDeviceManager` whose
/// device is on a different GPU than the encoder's (common on iGPU+dGPU
/// machines); the LUID comes from `MFT_ENUM_ADAPTER_LUID` on the MFT's
/// activate object. Falls back to the default adapter when no match is found.
fn ensure_d3d_manager(
    adapter_luid: Option<u64>,
) -> Result<(IMFDXGIDeviceManager, ID3D11Device, ID3D11DeviceContext), EncodeError> {
    unsafe {
        let mut device: Option<ID3D11Device> = None;
        let mut context: Option<ID3D11DeviceContext> = None;

        let adapter = match adapter_luid {
            Some(luid) => match find_adapter_by_luid(luid) {
                Ok(Some(adapter)) => {
                    let desc = adapter
                        .GetDesc1()
                        .map_err(|e| EncodeError::InitFailed(format!("GetDesc1: {e}")))?;
                    let name = desc.Description;
                    let name = String::from_utf16_lossy(&name);
                    let name = name.trim_end_matches('\0');
                    eprintln!(
                        "[h264] creating encoder D3D11 device on adapter \"{name}\" (LUID=0x{luid:x})"
                    );
                    Some(
                        adapter
                            .cast::<windows::Win32::Graphics::Dxgi::IDXGIAdapter>()
                            .map_err(|e| EncodeError::InitFailed(format!("adapter cast: {e}")))?,
                    )
                }
                Ok(None) => {
                    eprintln!(
                        "[h264] no DXGI adapter matches LUID=0x{luid:x}; using default adapter"
                    );
                    None
                }
                Err(e) => {
                    eprintln!("[h264] adapter enumeration failed ({e}); using default adapter");
                    None
                }
            },
            None => None,
        };

        let hr = D3D11CreateDevice(
            adapter.as_ref(),
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_VIDEO_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        );
        if let Err(e) = hr {
            return Err(EncodeError::InitFailed(format!(
                "D3D11CreateDevice for encoder: {e}"
            )));
        }
        let device = device.ok_or_else(|| {
            EncodeError::InitFailed("D3D11CreateDevice returned no device".into())
        })?;
        let context = context.ok_or_else(|| {
            EncodeError::InitFailed("D3D11CreateDevice returned no context".into())
        })?;

        let mut reset_token: u32 = 0;
        let mut manager: Option<IMFDXGIDeviceManager> = None;
        MFCreateDXGIDeviceManager(&mut reset_token, &mut manager)
            .map_err(|e| EncodeError::InitFailed(format!("MFCreateDXGIDeviceManager: {e}")))?;
        let manager = manager.ok_or_else(|| {
            EncodeError::InitFailed("MFCreateDXGIDeviceManager returned no manager".into())
        })?;

        let unknown: windows::core::IUnknown = device
            .cast()
            .map_err(|e| EncodeError::InitFailed(format!("device to IUnknown: {e}")))?;
        manager
            .ResetDevice(&unknown, reset_token)
            .map_err(|e| EncodeError::InitFailed(format!("DXGI ResetDevice: {e}")))?;

        Ok((manager, device, context))
    }
}

/// Find the DXGI adapter whose `AdapterLuid` matches `luid` (packed as
/// `LowPart` in the low 32 bits, `HighPart` in the high 32 bits, the layout
/// used by `MFT_ENUM_ADAPTER_LUID`). Returns `Ok(None)` when no adapter
/// matches.
fn find_adapter_by_luid(luid: u64) -> Result<Option<IDXGIAdapter1>, EncodeError> {
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, DXGI_ERROR_NOT_FOUND};

    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }
        .map_err(|e| EncodeError::InitFailed(format!("CreateDXGIFactory1: {e}")))?;

    let want_low = luid as u32;
    let want_high = (luid >> 32) as i32;
    let mut index = 0u32;
    loop {
        let adapter = match unsafe { factory.EnumAdapters1(index) } {
            Ok(a) => a,
            Err(e) if e.code() == DXGI_ERROR_NOT_FOUND => return Ok(None),
            Err(e) => {
                return Err(EncodeError::InitFailed(format!(
                    "EnumAdapters1({index}): {e}"
                )))
            }
        };
        match unsafe { adapter.GetDesc1() } {
            Ok(desc) => {
                if desc.AdapterLuid.LowPart == want_low && desc.AdapterLuid.HighPart == want_high {
                    return Ok(Some(adapter));
                }
            }
            Err(_) => {}
        }
        index += 1;
    }
}

fn create_ms_software_encoder() -> Result<IMFTransform, EncodeError> {
    // SAFETY: The Microsoft H.264 encoder is an in-proc COM server.
    unsafe {
        let transform: IMFTransform =
            CoCreateInstance(&CLSID_MSH264EncoderMFT, None, CLSCTX_INPROC_SERVER).map_err(|e| {
                EncodeError::InitFailed(format!("CoCreateInstance H.264 encoder MFT: {e}"))
            })?;
        Ok(transform)
    }
}

// ---------------------------------------------------------------------------
// H.264 encoder
// ---------------------------------------------------------------------------

/// Maximum D3D11 texture-pool slots for the async MFT path. Bounds GPU memory:
/// NV12 1080p ≈ 3 MB/slot, 4K ≈ 12 MB/slot → ≤ 192 MB at 4K. Larger than the
/// encoder's internal pipeline depth so a lagging MFT never starves the pump.
const MAX_ASYNC_POOL_SLOTS: usize = 16;

/// Processing model used by an `MfH264Encoder`.
enum Processing {
    /// Direct `ProcessInput`/`ProcessOutput` loop (Microsoft software encoder
    /// and synchronous hardware MFTs).
    Sync,
    /// Event-driven async MFT model (AMD/NVIDIA hardware encoders): input is
    /// delivered in response to `METransformNeedInput` events as D3D11 NV12
    /// textures, output is drained on `METransformHaveOutput`.
    Async(AsyncState),
}

/// A frame queued for delivery to an async MFT.
struct QueuedInput {
    /// Pool slot index holding the uploaded texture (see `AsyncState.pool`).
    slot: usize,
    /// `IMFSample` wrapping the texture via `MFCreateDXGISurfaceBuffer`.
    sample: IMFSample,
    /// Capture timestamp of the frame, threaded through to the ring buffer.
    timestamp: std::time::Instant,
}

/// State for the async MFT processing model.
struct AsyncState {
    /// Event source driving `METransformNeedInput` / `METransformHaveOutput`.
    events: IMFMediaEventGenerator,
    /// Optional `ICodecAPI` used to force keyframes.
    codec_api: Option<ICodecAPI>,
    /// Device + context for uploading NV12 into the texture pool. The device
    /// creates textures; the context Maps/Unmaps them (same D3D11 device the
    /// DXGI device manager was reset with, so the MFT can read our textures).
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    /// D3D11 NV12 textures, one per pool slot.
    pool: Vec<Option<ID3D11Texture2D>>,
    /// Slots that currently hold an uploaded-but-unconsumed frame.
    pending: VecDeque<QueuedInput>,
    /// Slots handed to the MFT awaiting their output, in submission order with
    /// their capture timestamps (output lags input by encoder latency).
    in_flight: VecDeque<(usize, std::time::Instant)>,
    /// Free pool slots not currently referenced by the MFT.
    free: Vec<usize>,
    /// The MFT signaled `METransformNeedInput` while we had no pending frame;
    /// satisfy it on the next `encode_frame`.
    need_input_pending: bool,
    /// Diagnostics counters (surfaced by the diag test).
    diag_need_input: u32,
    diag_have_output: u32,
    diag_notaccepting: u32,
    diag_inputs_delivered: u32,
    diag_outputs_empty: u32,
    diag_other_events: u32,
}

/// Media Foundation H.264 Video Encoder MFT wrapper.
pub struct MfH264Encoder {
    transform: IMFTransform,
    processing: Processing,
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
    /// Reusable input sample + buffer, avoiding per-frame MF allocations.
    input_sample: Option<IMFSample>,
    input_buffer: Option<IMFMediaBuffer>,
    /// Set by `request_keyframe()`; the next encoded frame is forced to be a
    /// keyframe (IDR). Cleared after the next `encode_frame` call.
    force_keyframe: bool,
    /// D3D11 device + context + DXGI device manager kept alive for the MFT's
    /// lifetime. Hardware MFTs hold the manager pointer passed via
    /// `MFT_MESSAGE_SET_D3D_MANAGER` without necessarily AddRef'ing it, so the
    /// app must keep it valid.
    #[allow(dead_code)]
    d3d_resources: Option<(IMFDXGIDeviceManager, ID3D11Device, ID3D11DeviceContext)>,
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
    /// * `width`, `height` â€” video dimensions (input must be NV12 at this res)
    /// * `fps` â€” frame rate
    /// * `bitrate_kbps` â€” target average bitrate in kilobits / sec
    /// * `keyframe_interval` â€” GOP size (keyframe every N frames)
    pub fn new(
        width: u32,
        height: u32,
        fps: u32,
        bitrate_kbps: u32,
        keyframe_interval: u32,
    ) -> Result<Self, EncodeError> {
        Self::with_profile(
            width,
            height,
            fps,
            bitrate_kbps,
            keyframe_interval,
            Some(77),
        )
    }

    /// Like [`Self::new`] but with an explicit `MF_MT_MPEG2_PROFILE`
    /// (`Some(66)` = Baseline, `Some(77)` = Main, `Some(100)` = High,
    /// `None` = leave unset). Some hardware encoders reject Baseline, so the
    /// profile is tunable for diagnosis.
    pub fn with_profile(
        width: u32,
        height: u32,
        fps: u32,
        bitrate_kbps: u32,
        keyframe_interval: u32,
        profile: Option<u32>,
    ) -> Result<Self, EncodeError> {
        ensure_mf()?;

        // Profile candidates tried in order. Hardware MFTs are picky about the
        // negotiated profile; iterate until one accepts. The caller's requested
        // profile wins first, then Main (safest default), unset, Baseline, High.
        let requested = profile;
        let candidates: &[Option<u32>] = &[requested, Some(77), None, Some(66), Some(100)];
        let mut seen = Vec::new();
        let mut ordered: Vec<Option<u32>> = Vec::new();
        for c in candidates {
            if !seen.contains(c) {
                seen.push(*c);
                ordered.push(*c);
            }
        }

        // Stage 1: hardware encoder (NVENC / AMF / QuickSync). The DXGI device
        // manager is attached inside `from_transform`, which is what makes most
        // hardware MFTs accept CPU-supplied NV12 input. Every enumerated
        // candidate is tried (sync MFTs via the direct model, async MFTs such
        // as AMD/NVIDIA via the event-driven model), in enumeration order.
        let mut tried_hw = false;
        let mut last_hw_err: Option<String> = None;
        match enumerate_h264_encoders() {
            Ok(candidates) => {
                for cand in &candidates {
                    tried_hw = true;
                    for p in &ordered {
                        match Self::from_transform(
                            cand.transform.clone(),
                            width,
                            height,
                            fps,
                            bitrate_kbps,
                            keyframe_interval,
                            *p,
                            cand.adapter_luid,
                        ) {
                            Ok(enc) => {
                                eprintln!(
                                    "[h264] hardware H.264 encoder active: \"{}\" \
                                     (async={}, profile {p:?})",
                                    cand.name, cand.is_async
                                );
                                return Ok(enc);
                            }
                            Err(e) => {
                                eprintln!(
                                    "[h264] hardware encoder \"{}\" rejected profile {p:?}: {e}",
                                    cand.name
                                );
                                last_hw_err = Some(e.to_string());
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "[prism] hardware H.264 encoder enumeration failed ({e}); \
                     falling back to Microsoft software encoder"
                );
            }
        }
        if tried_hw {
            eprintln!(
                "[prism] hardware H.264 encoders rejected all profiles \
                 (last: {:?}); falling back to Microsoft software encoder",
                last_hw_err
            );
        } else {
            eprintln!(
                "[prism] no hardware H.264 encoder available; \
                 falling back to Microsoft software encoder"
            );
        }

        // Stage 2 (last resort): Microsoft software encoder. Kept so clip
        // saving still works on machines with no hardware H.264 MFT; it costs
        // CPU, which is why hardware is preferred.
        let soft = create_ms_software_encoder()?;
        for p in &ordered {
            match Self::from_transform(
                soft.clone(),
                width,
                height,
                fps,
                bitrate_kbps,
                keyframe_interval,
                *p,
                None,
            ) {
                Ok(enc) => {
                    eprintln!("[h264] Microsoft software encoder active (profile {p:?})");
                    return Ok(enc);
                }
                Err(e) => {
                    eprintln!("[h264] software encoder rejected profile {p:?}: {e}");
                }
            }
        }
        Err(EncodeError::InitFailed(
            "H.264 encoder failed to negotiate any profile".into(),
        ))
    }

    /// Perform full MFT negotiation over a caller-provided transform.
    /// Public only for diagnostics; production code uses [`Self::new`].
    ///
    /// `adapter_luid` is the `MFT_ENUM_ADAPTER_LUID` of the hardware the
    /// transform is bound to (see [`enumerate_h264_encoders`]); the D3D11
    /// device created for the DXGI manager is placed on that adapter so the
    /// MFT accepts it. Pass `None` for software encoders.
    #[allow(dead_code)]
    pub fn from_transform(
        transform: IMFTransform,
        width: u32,
        height: u32,
        fps: u32,
        bitrate_kbps: u32,
        keyframe_interval: u32,
        profile: Option<u32>,
        adapter_luid: Option<u64>,
    ) -> Result<Self, EncodeError> {
        unsafe {
            let is_async = Self::transform_is_async(&transform);

            // Async hardware MFTs (AMD/NVIDIA) are locked by default and reject
            // SetInputType/SetOutputType with MF_E_TRANSFORM_ASYNC_LOCKED until
            // the client sets MF_TRANSFORM_ASYNC_UNLOCK=TRUE on their attributes.
            if is_async {
                if let Ok(attrs) = transform.GetAttributes() {
                    match attrs.SetUINT32(&MF_TRANSFORM_ASYNC_UNLOCK, 1) {
                        Ok(()) => {
                            eprintln!("[h264] async MFT unlocked (MF_TRANSFORM_ASYNC_UNLOCK)")
                        }
                        Err(e) => eprintln!(
                            "[h264] async MFT could not set MF_TRANSFORM_ASYNC_UNLOCK: {e}"
                        ),
                    }
                }
            }

            // ------ Attach the DXGI device manager BEFORE negotiating types ------
            // Hardware H.264 MFTs require an IMFDXGIDeviceManager to accept
            // CPU-supplied NV12 input; without one their SetOutputType fails
            // with "The input type is not supported for D3D device"
            // (0xC00D6D76). The MS software encoder ignores this message.
            let d3d_resources = match ensure_d3d_manager(adapter_luid) {
                Ok((manager, device, context)) => {
                    let manager_ptr = manager.as_raw() as usize;
                    match transform.ProcessMessage(MFT_MESSAGE_SET_D3D_MANAGER, manager_ptr) {
                        Ok(()) => {
                            eprintln!("[h264] D3D11 device manager attached");
                            Some((manager, device, context))
                        }
                        Err(e) => {
                            eprintln!(
                                "[h264] MFT rejected D3D11 device manager ({e}); continuing without it"
                            );
                            None
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "[h264] failed to create D3D11 device manager ({e}); continuing without it"
                    );
                    None
                }
            };

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
            if let Some(profile) = profile {
                output_type
                    .SetUINT32(&MF_MT_MPEG2_PROFILE, profile)
                    .map_err(|e| EncodeError::InitFailed(format!("SetUINT32 profile: {e}")))?;
            }
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

            if is_async {
                // Low-latency hint keeps encode latency to a frame or two.
                if let Ok(attrs) = transform.GetAttributes() {
                    attrs.SetUINT32(&MF_LOW_LATENCY, 1).ok();
                }
                // COMMAND_FLUSH discards stale async events queued during setup
                // (e.g. spurious NeedInput emitted during type negotiation).
                transform.ProcessMessage(MFT_MESSAGE_COMMAND_FLUSH, 0).ok();
            }

            transform
                .ProcessMessage(MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, 0)
                .map_err(|e| EncodeError::InitFailed(format!("Begin streaming: {e}")))?;
            transform
                .ProcessMessage(MFT_MESSAGE_NOTIFY_START_OF_STREAM, 0)
                .map_err(|e| EncodeError::InitFailed(format!("Start stream: {e}")))?;

            let processing = if is_async {
                // Async MFTs are driven by their event source: we feed NV12
                // input on METransformNeedInput and drain compressed output on
                // METransformHaveOutput. Keyframes are forced via ICodecAPI.
                let events: IMFMediaEventGenerator = transform.cast().map_err(|e| {
                    EncodeError::InitFailed(format!("QI IMFMediaEventGenerator: {e}"))
                })?;
                let codec_api: Option<ICodecAPI> = transform.cast().ok();
                let (device, context) = d3d_resources
                    .as_ref()
                    .map(|(_, d, c)| (d.clone(), c.clone()))
                    .ok_or_else(|| {
                        EncodeError::InitFailed(
                            "async hardware MFT requires an attached D3D11 device manager".into(),
                        )
                    })?;
                Processing::Async(AsyncState {
                    events,
                    codec_api,
                    device,
                    context,
                    pool: Vec::new(),
                    pending: VecDeque::new(),
                    in_flight: VecDeque::new(),
                    free: Vec::new(),
                    need_input_pending: false,
                    diag_need_input: 0,
                    diag_have_output: 0,
                    diag_notaccepting: 0,
                    diag_inputs_delivered: 0,
                    diag_outputs_empty: 0,
                    diag_other_events: 0,
                })
            } else {
                Processing::Sync
            };

            Ok(Self {
                transform,
                processing,
                frame_index: 0,
                timescale: fps as i64,
                width,
                height,
                sps: Vec::new(),
                pps: Vec::new(),
                sps_pps_ready: false,
                input_sample: None,
                input_buffer: None,
                force_keyframe: false,
                d3d_resources,
            })
        }
    }

    /// Whether the given MFT advertises the async transform attribute. Async
    /// hardware encoders (AMD/NVIDIA) require the event-driven processing model.
    fn transform_is_async(transform: &IMFTransform) -> bool {
        unsafe { transform.GetAttributes() }
            .and_then(|attrs| unsafe { attrs.GetUINT32(&MF_TRANSFORM_ASYNC) })
            .unwrap_or(0)
            != 0
    }

    /// Encode a single NV12 frame.
    ///
    /// Returns zero or more [`EncodedPacket`] — usually one per frame, but the
    /// MFT may batch multiple frames internally before producing output. Each
    /// packet carries the capture `timestamp` of the frame it was encoded from
    /// (attributed from the input FIFO on the async path, which lags by the
    /// encoder's internal latency).
    pub fn encode_frame(
        &mut self,
        nv12: &[u8],
        timestamp: std::time::Instant,
    ) -> Result<Vec<EncodedPacket>, EncodeError> {
        let expected_size = nv12_packed_size(self.width, self.height);
        if nv12.len() != expected_size {
            return Err(EncodeError::EncodeFailed(format!(
                "Invalid NV12 buffer size: got {} expected {}",
                nv12.len(),
                expected_size
            )));
        }
        if matches!(self.processing, Processing::Async(_)) {
            unsafe { self.encode_frame_async(nv12, timestamp) }
        } else {
            unsafe { self.encode_frame_sync(nv12, timestamp) }
        }
    }

    /// Synchronous processing model: write NV12 into a reusable memory buffer,
    /// `ProcessInput`, then drain `ProcessOutput` until
    /// `MF_E_TRANSFORM_NEED_MORE_INPUT`. Used by the Microsoft software encoder
    /// and synchronous hardware MFTs (e.g. Intel QuickSync).
    unsafe fn encode_frame_sync(
        &mut self,
        nv12: &[u8],
        timestamp: std::time::Instant,
    ) -> Result<Vec<EncodedPacket>, EncodeError> {
        unsafe {
            let expected_size = nv12_packed_size(self.width, self.height);

            // ------ Create/reuse input sample ------
            // Reusing a single IMFSample + IMFMediaBuffer across frames avoids
            // two MF allocations (and their refcount churn) per captured frame.
            let (sample, buffer) = match (self.input_sample.as_ref(), self.input_buffer.as_ref()) {
                (Some(s), Some(b)) => (s.clone(), b.clone()),
                _ => {
                    let buffer: IMFMediaBuffer = MFCreateMemoryBuffer(expected_size as u32)
                        .map_err(|e| {
                            EncodeError::EncodeFailed(format!("CreateMemoryBuffer: {e}"))
                        })?;
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

            std::ptr::copy_nonoverlapping(nv12.as_ptr(), ptr, expected_size);

            buffer
                .SetCurrentLength(expected_size as u32)
                .map_err(|e| EncodeError::EncodeFailed(format!("SetCurrentLength: {e}")))?;

            buffer
                .Unlock()
                .map_err(|e| EncodeError::EncodeFailed(format!("Unlock buffer: {e}")))?;

            let duration_100ns = 10_000_000 / self.timescale;
            let timestamp_100ns = self.frame_index * duration_100ns;

            sample
                .SetSampleTime(timestamp_100ns)
                .map_err(|e| EncodeError::EncodeFailed(format!("SetSampleTime: {e}")))?;
            sample
                .SetSampleDuration(duration_100ns)
                .map_err(|e| EncodeError::EncodeFailed(format!("SetSampleDuration: {e}")))?;

            // Mark this frame as a sync point when it's the first frame or when a
            // keyframe was explicitly requested (clip-save retry). The input
            // sample is reused across frames, so the attribute must be rewritten
            // every frame â€” a stale CleanPoint=1 on every input would make the
            // encoder force IDRs on (or misbehave with) every frame.
            let request_key = self.frame_index == 0 || self.force_keyframe;
            sample
                .SetUINT32(&MFSampleExtension_CleanPoint, request_key as u32)
                .ok();
            self.force_keyframe = false;

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
                        let raw = collect_sample_bytes(out_sample, None)?;
                        let avcc = h264_to_avcc(raw)?;
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
                            timestamp,
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

    /// Async processing model: pump the MFT event queue, upload NV12 into a
    /// pooled D3D11 texture, deliver it on `METransformNeedInput`, and drain
    /// compressed output on `METransformHaveOutput`.
    unsafe fn encode_frame_async(
        &mut self,
        nv12: &[u8],
        timestamp: std::time::Instant,
    ) -> Result<Vec<EncodedPacket>, EncodeError> {
        let trace = std::env::var("PRISM_ASYNC_TRACE").as_deref() == Ok("1");
        if trace {
            eprintln!("[trace] encode_frame_async frame {}", self.frame_index);
        }
        // Hardware encoders ignore the input sample's CleanPoint attribute, so
        // a requested IDR is forced directly on the codec instead.
        {
            let Processing::Async(state) = &mut self.processing else {
                unreachable!();
            };
            if self.force_keyframe {
                if let Some(codec_api) = state.codec_api.as_ref() {
                    let mut variant = VARIANT::default();
                    (*variant.Anonymous.Anonymous).vt = VT_UI4;
                    (*variant.Anonymous.Anonymous).Anonymous.ulVal = 1;
                    codec_api
                        .SetValue(&CODECAPI_AVEncVideoForceKeyFrame, &variant)
                        .ok();
                }
                self.force_keyframe = false;
            }
        }

        let mut packets: Vec<EncodedPacket> = Vec::new();

        // Drain queued events first so texture slots freed by completed frames
        // are reusable for the upcoming upload.
        self.async_pump_events(&mut packets)?;

        let slot = self.async_upload_frame(nv12)?;
        let sample = self.async_make_input_sample(slot)?;
        {
            let Processing::Async(state) = &mut self.processing else {
                unreachable!();
            };
            state.pending.push_back(QueuedInput {
                slot,
                sample,
                timestamp,
            });
            if trace {
                eprintln!(
                    "[trace]   queued slot={slot} pending={} need_input_pending={}",
                    state.pending.len(),
                    state.need_input_pending
                );
            }
            // The MFT may be waiting on input from an earlier NeedInput event
            // (it does not re-emit while starved) — deliver immediately.
            if state.need_input_pending {
                if let Some(qi) = state.pending.pop_front() {
                    match self.transform.ProcessInput(0, &qi.sample, 0) {
                        Ok(()) => {
                            state.in_flight.push_back((qi.slot, qi.timestamp));
                            state.need_input_pending = false;
                        }
                        Err(e) if e.code() == MF_E_NOTACCEPTING => {
                            state.pending.push_front(qi);
                        }
                        Err(e) => {
                            return Err(EncodeError::EncodeFailed(format!(
                                "async ProcessInput: {e}"
                            )))
                        }
                    }
                }
            }
        }

        // Pump again to deliver the just-queued input and drain its output.
        self.async_pump_events(&mut packets)?;

        // Fallback: if SPS/PPS still not found in the bitstream, try the output
        // media type (some GPU drivers omit parameter sets from the bitstream).
        if !self.sps_pps_ready {
            if capture_sps_pps_from_media_type(&self.transform, &mut self.sps, &mut self.pps)
                .is_ok()
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

    /// Pump pending async MFT events until the queue is drained. Feeds queued
    /// inputs on `METransformNeedInput` and drains compressed output on
    /// `METransformHaveOutput`.
    fn async_pump_events(&mut self, packets: &mut Vec<EncodedPacket>) -> Result<(), EncodeError> {
        unsafe {
            let Self {
                transform,
                sps,
                pps,
                sps_pps_ready,
                processing,
                ..
            } = self;
            let Processing::Async(state) = processing else {
                unreachable!();
            };

            loop {
                let trace = std::env::var("PRISM_ASYNC_TRACE").as_deref() == Ok("1");
                let event = match state.events.GetEvent(MF_EVENT_FLAG_NO_WAIT) {
                    Ok(event) => event,
                    Err(e) if e.code() == MF_E_NO_EVENTS_AVAILABLE => {
                        if trace {
                            eprintln!("[trace]     pump: no events");
                        }
                        break;
                    }
                    Err(e) => {
                        return Err(EncodeError::EncodeFailed(format!("async GetEvent: {e}")))
                    }
                };

                let event_type = event
                    .GetType()
                    .map_err(|e| EncodeError::EncodeFailed(format!("async GetEventType: {e}")))?;
                if trace {
                    eprintln!("[trace]     pump: event type {event_type}");
                }

                if event_type == METransformNeedInput.0 as u32 {
                    state.diag_need_input += 1;
                    match state.pending.pop_front() {
                        Some(qi) => match transform.ProcessInput(0, &qi.sample, 0) {
                            Ok(()) => {
                                state.in_flight.push_back((qi.slot, qi.timestamp));
                                state.need_input_pending = false;
                                state.diag_inputs_delivered += 1;
                            }
                            Err(e) if e.code() == MF_E_NOTACCEPTING => {
                                // MFT is full; requeue and wait for a later NeedInput.
                                state.pending.push_front(qi);
                                state.diag_notaccepting += 1;
                            }
                            Err(e) => {
                                return Err(EncodeError::EncodeFailed(format!(
                                    "async ProcessInput: {e}"
                                )))
                            }
                        },
                        None => {
                            state.need_input_pending = true;
                        }
                    }
                } else if event_type == METransformHaveOutput.0 as u32 {
                    state.diag_have_output += 1;
                    let mut output = create_output_buffer_for(transform)?;
                    let mut status: u32 = 0;
                    let result =
                        transform.ProcessOutput(0, std::slice::from_mut(&mut output), &mut status);

                    let output_result: Result<Option<EncodedPacket>, EncodeError> = (|| {
                        if let Err(e) = &result {
                            if e.code() == MF_E_TRANSFORM_STREAM_CHANGE {
                                if let Ok(new_type) = transform.GetOutputAvailableType(0, 0) {
                                    transform.SetOutputType(0, &new_type, 0).ok();
                                }
                                return Ok(None);
                            }
                            return Err(EncodeError::EncodeFailed(format!(
                                "async ProcessOutput: {e}"
                            )));
                        }
                        let Some(out_sample) = output.pSample.as_ref() else {
                            state.diag_outputs_empty += 1;
                            return Ok(None);
                        };
                        let timestamp = state
                            .in_flight
                            .pop_front()
                            .map(|(slot, ts)| {
                                // The encoder is done with this input texture once its
                                // output is delivered — recycle the slot.
                                state.free.push(slot);
                                ts
                            })
                            .unwrap_or_else(|| std::time::Instant::now());
                        build_packet_from_sample(
                            out_sample,
                            Some(&state.context),
                            sps,
                            pps,
                            sps_pps_ready,
                            timestamp,
                        )
                    })(
                    );
                    release_output_buffer(&mut output);
                    if let Some(packet) = output_result? {
                        packets.push(packet);
                    }
                } else {
                    state.diag_other_events += 1;
                }
            }
            Ok(())
        }
    }

    /// Upload packed NV12 into a free pool texture, returning its slot index.
    /// The pool is lazily grown up to [`MAX_ASYNC_POOL_SLOTS`].
    unsafe fn async_upload_frame(&mut self, nv12: &[u8]) -> Result<usize, EncodeError> {
        let Self {
            width,
            height,
            processing,
            ..
        } = self;
        let Processing::Async(state) = processing else {
            unreachable!();
        };

        let slot = if let Some(slot) = state.free.pop() {
            slot
        } else if state.pool.len() < MAX_ASYNC_POOL_SLOTS {
            let desc = D3D11_TEXTURE2D_DESC {
                Width: *width,
                Height: *height,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_NV12,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                CPUAccessFlags: 0,
                MiscFlags: 0,
            };
            let mut texture: Option<ID3D11Texture2D> = None;
            state
                .device
                .CreateTexture2D(&desc, None, Some(&mut texture))
                .map_err(|e| EncodeError::EncodeFailed(format!("CreateTexture2D pool: {e}")))?;
            let texture = texture.ok_or_else(|| {
                EncodeError::EncodeFailed("CreateTexture2D pool returned no texture".into())
            })?;
            state.pool.push(Some(texture));
            state.pool.len() - 1
        } else {
            return Err(EncodeError::EncodeFailed(
                "async encoder texture pool exhausted".into(),
            ));
        };

        let texture = state.pool[slot]
            .as_ref()
            .ok_or_else(|| EncodeError::EncodeFailed("async pool slot empty".into()))?;

        // NV12 textures are GPU-only (no CPU access), so CPU uploads must go
        // through UpdateSubresource with a tightly-packed Y-then-UV source.
        // SrcRowPitch = Y row pitch (width), SrcDepthPitch = offset to UV plane.
        let y_bytes = ((*width as usize) * (*height as usize)) as u32;
        state.context.UpdateSubresource(
            texture,
            0,
            None,
            nv12.as_ptr() as *const core::ffi::c_void,
            *width,
            y_bytes,
        );
        // Submit the queued GPU copy before the MFT reads the texture — the MFT
        // may use its own device context, and D3D11 does not order work across
        // contexts on the same device without an explicit flush.
        state.context.Flush();
        Ok(slot)
    }

    /// Wrap a pooled texture in an `IMFSample` via `MFCreateDXGISurfaceBuffer`
    /// so the async MFT reads the frame straight from GPU memory.
    unsafe fn async_make_input_sample(&mut self, slot: usize) -> Result<IMFSample, EncodeError> {
        let Processing::Async(state) = &mut self.processing else {
            unreachable!();
        };
        let texture = state.pool[slot]
            .as_ref()
            .ok_or_else(|| EncodeError::EncodeFailed("async pool slot empty".into()))?;

        let unk: windows::core::IUnknown = texture
            .cast()
            .map_err(|e| EncodeError::EncodeFailed(format!("texture to IUnknown: {e}")))?;

        let buffer: IMFMediaBuffer =
            MFCreateDXGISurfaceBuffer(&ID3D11Texture2D::IID, &unk, 0, false).map_err(|e| {
                EncodeError::EncodeFailed(format!("MFCreateDXGISurfaceBuffer: {e}"))
            })?;
        let sample: IMFSample = MFCreateSample()
            .map_err(|e| EncodeError::EncodeFailed(format!("MFCreateSample: {e}")))?;
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
        Ok(sample)
    }

    /// Create the output descriptor required by this MFT. Some encoders provide
    /// their own samples; others require a caller-allocated media buffer.
    ///
    /// Per MSDN, `MFT_OUTPUT_STREAM_PROVIDES_SAMPLES` means the caller must pass
    /// `pSample = NULL`. `MFT_OUTPUT_STREAM_CAN_PROVIDE_SAMPLES` only means the
    /// MFT *may* provide samples; the caller still allocates one. The Microsoft
    /// software H.264 encoder sets CAN_PROVIDE without PROVIDES, so passing NULL
    /// there would silently starve the output.
    unsafe fn create_output_buffer(&self) -> Result<MFT_OUTPUT_DATA_BUFFER, EncodeError> {
        create_output_buffer_for(&self.transform)
    }
}

/// Create the output descriptor required by this MFT. Some encoders provide
/// their own samples; others require a caller-allocated media buffer.
///
/// Per MSDN, `MFT_OUTPUT_STREAM_PROVIDES_SAMPLES` means the caller must pass
/// `pSample = NULL`. `MFT_OUTPUT_STREAM_CAN_PROVIDE_SAMPLES` only means the
/// MFT *may* provide samples; the caller still allocates one. The Microsoft
/// software H.264 encoder sets CAN_PROVIDE without PROVIDES, so passing NULL
/// there would silently starve the output.
unsafe fn create_output_buffer_for(
    transform: &IMFTransform,
) -> Result<MFT_OUTPUT_DATA_BUFFER, EncodeError> {
    let info = transform
        .GetOutputStreamInfo(0)
        .map_err(|e| EncodeError::EncodeFailed(format!("GetOutputStreamInfo: {e}")))?;

    if info.dwFlags & MFT_OUTPUT_STREAM_PROVIDES_SAMPLES.0 as u32 != 0 {
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

/// The generated bindings use `ManuallyDrop` for COM pointers in this FFI
/// struct, so release them explicitly after every `ProcessOutput` call.
unsafe fn release_output_buffer(output: &mut MFT_OUTPUT_DATA_BUFFER) {
    std::mem::ManuallyDrop::drop(&mut output.pSample);
    std::mem::ManuallyDrop::drop(&mut output.pEvents);
}

// ---------------------------------------------------------------------------
// Helper: collect bytes from an output sample
// ---------------------------------------------------------------------------

/// Convert one output sample into an [`EncodedPacket`], capturing SPS/PPS when
/// they first appear. `context` enables the DXGI fallback for GPU-resident
/// output buffers (async hardware MFTs). Shared by the sync and async paths.
unsafe fn build_packet_from_sample(
    sample: &IMFSample,
    context: Option<&ID3D11DeviceContext>,
    sps: &mut Vec<u8>,
    pps: &mut Vec<u8>,
    sps_pps_ready: &mut bool,
    timestamp: std::time::Instant,
) -> Result<Option<EncodedPacket>, EncodeError> {
    let raw = collect_sample_bytes(sample, context)?;
    let avcc = h264_to_avcc(raw)?;
    let clean_point = is_keyframe(sample);
    let has_idr = avcc_contains_idr(&avcc);
    let is_key = clean_point || has_idr;

    if has_idr && !clean_point {
        eprintln!("[prism] marked H.264 packet as sync from its IDR NAL (MFT omitted CleanPoint)");
    }

    // MFTs may return either Annex B or AVCC samples. Normalize
    // first so both packet storage and parameter-set parsing use the
    // AVCC format required by the MP4 muxer.
    if !*sps_pps_ready {
        capture_sps_pps_from_avcc(&avcc, sps, pps);
        if !sps.is_empty() && !pps.is_empty() {
            *sps_pps_ready = true;
        }
    }

    Ok(Some(EncodedPacket {
        data: avcc,
        is_sync: is_key,
        timestamp,
    }))
}

/// Copy the output sample's bytes into a `Vec`. Hardware encoders normally
/// return a CPU-accessible buffer; GPU-resident (DXGI) output buffers are
/// copied out through the provided D3D11 context.
unsafe fn collect_sample_bytes(
    sample: &IMFSample,
    context: Option<&ID3D11DeviceContext>,
) -> Result<Vec<u8>, EncodeError> {
    let buffer: IMFMediaBuffer = sample
        .GetBufferByIndex(0)
        .map_err(|e| EncodeError::EncodeFailed(format!("GetBufferByIndex: {e}")))?;

    let mut ptr: *mut u8 = std::ptr::null_mut();
    let mut max_len: u32 = 0;
    let mut cur_len: u32 = 0;

    if buffer
        .Lock(&mut ptr, Some(&mut max_len), Some(&mut cur_len))
        .is_ok()
    {
        let data = std::slice::from_raw_parts(ptr, cur_len as usize).to_vec();
        buffer.Unlock().ok();
        return Ok(data);
    }

    match context {
        Some(context) => collect_dxgi_sample_bytes(&buffer, context),
        None => Err(EncodeError::EncodeFailed(
            "output buffer is not CPU-accessible (DXGI buffer with no context)".into(),
        )),
    }
}

/// Copy bytes out of a GPU-resident (DXGI) output buffer by mapping the D3D11
/// NV12 texture through the encoder's device context and packing the rows
/// tightly (removing row-pitch padding).
unsafe fn collect_dxgi_sample_bytes(
    buffer: &IMFMediaBuffer,
    context: &ID3D11DeviceContext,
) -> Result<Vec<u8>, EncodeError> {
    let dxgi: IMFDXGIBuffer = buffer
        .cast()
        .map_err(|e| EncodeError::EncodeFailed(format!("QI IMFDXGIBuffer: {e}")))?;

    let subresource = dxgi
        .GetSubresourceIndex()
        .map_err(|e| EncodeError::EncodeFailed(format!("GetSubresourceIndex: {e}")))?;

    let mut resource_ptr: *mut core::ffi::c_void = std::ptr::null_mut();
    dxgi.GetResource(&ID3D11Texture2D::IID, &mut resource_ptr)
        .map_err(|e| EncodeError::EncodeFailed(format!("GetResource: {e}")))?;
    // SAFETY: GetResource returns a new reference to the underlying texture.
    let texture: ID3D11Texture2D = windows::core::Interface::from_raw(resource_ptr);

    let mut desc = D3D11_TEXTURE2D_DESC::default();
    texture.GetDesc(&mut desc);

    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    context
        .Map(&texture, subresource, D3D11_MAP_READ, 0, Some(&mut mapped))
        .map_err(|e| EncodeError::EncodeFailed(format!("Map output texture: {e}")))?;

    let width = desc.Width as usize;
    let height = desc.Height as usize;
    let y_pitch = mapped.RowPitch as usize;
    let uv_pitch = y_pitch / 2;
    let uv_width = width.div_ceil(2) * 2;
    let uv_rows = height.div_ceil(2);

    let mut data = Vec::with_capacity(width * height + uv_rows * uv_width);
    let base = mapped.pData.cast::<u8>();
    for row in 0..height {
        data.extend_from_slice(std::slice::from_raw_parts(base.add(row * y_pitch), width));
    }
    let uv_base = base.add(height * y_pitch);
    for row in 0..uv_rows {
        data.extend_from_slice(std::slice::from_raw_parts(
            uv_base.add(row * uv_pitch),
            uv_width,
        ));
    }

    context.Unmap(&texture, subresource);
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
// Annex B â†’ AVCC conversion
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

/// Normalize a Media Foundation H.264 packet to AVCC, consuming the input
/// buffer. Hardware MFTs may emit either Annex B or AVCC depending on the
/// driver and negotiated output type.
///
/// Fast path: avoids an O(n) `is_valid_avcc` scan AND a buffer copy for the
/// common case where the MFT already outputs AVCC (modern drivers).
fn h264_to_avcc(data: Vec<u8>) -> Result<Vec<u8>, EncodeError> {
    if !looks_like_annex_b(&data) {
        // Common case: MFT outputs AVCC directly â€” skip O(n) validation scan.
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
        return Ok(data);
    }

    let avcc = annex_b_to_avcc(&data);
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

    /// Request that the next encoded frame be a keyframe (IDR). Used by clip
    /// save so a fresh decodable sync point lands in the ring buffer quickly.
    pub fn request_keyframe(&mut self) {
        self.force_keyframe = true;
    }

    /// Diagnostics: async-path event/counter totals, for the diag test.
    /// Returns `None` when running in the synchronous model.
    #[allow(dead_code)]
    pub fn diag_async_stats(&self) -> Option<[u32; 6]> {
        match &self.processing {
            Processing::Async(s) => Some([
                s.diag_need_input,
                s.diag_have_output,
                s.diag_inputs_delivered,
                s.diag_notaccepting,
                s.diag_outputs_empty,
                s.diag_other_events,
            ]),
            Processing::Sync => None,
        }
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
    fn nv12_packed_size_matches_half_sized_uv_for_even_dims() {
        assert_eq!(nv12_packed_size(1920, 1080), 1920 * 1080 * 3 / 2);
        assert_eq!(nv12_packed_size(3840, 2160), 3840 * 2160 * 3 / 2);
    }

    #[test]
    fn nv12_packed_size_handles_odd_dimensions() {
        // ceil-w/2 * ceil-h/2 * 2 is larger than w*h/2, so the strict `!=`
        // guard must use the packed size or odd-resolution frames get rejected.
        let w = 1367u32;
        let h = 768u32;
        let packed = nv12_packed_size(w, h);
        assert_eq!(
            packed,
            (w * h) as usize + ((w / 2 + 1) * (h / 2) * 2) as usize
        );
        assert!(packed > (w * h * 3 / 2) as usize);
    }

    #[test]
    fn preserves_avcc_samples_and_captures_parameter_sets() {
        let sps = [0x67, 0x42, 0x00, 0x1E];
        let pps = [0x68, 0xCE, 0x06, 0xE2];
        let idr = [0x65, 0x88, 0x84];
        let packet = avcc(&[&sps, &pps, &idr]);

        let normalized = h264_to_avcc(packet.clone()).unwrap();
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

        let normalized = h264_to_avcc(annex_b).unwrap();
        let mut found_sps = Vec::new();
        let mut found_pps = Vec::new();
        capture_sps_pps_from_avcc(&normalized, &mut found_sps, &mut found_pps);

        assert_eq!(normalized, avcc(&[&sps, &pps, &idr]));
        assert_eq!(found_sps, avcc(&[&sps]));
        assert_eq!(found_pps, avcc(&[&pps]));
    }

    #[test]
    fn rejects_malformed_h264_packets() {
        let error = h264_to_avcc(vec![0, 0, 0, 8, 0x65]).unwrap_err();

        assert!(error.to_string().contains("invalid H.264 packet"));
    }

    #[test]
    fn detects_idr_keyframes_without_clean_point_metadata() {
        let p_slice = [0x41, 0x9A, 0x22];
        let idr = [0x65, 0x88, 0x84];

        assert!(!avcc_contains_idr(&avcc(&[&p_slice])));
        assert!(avcc_contains_idr(&avcc(&[&p_slice, &idr])));
    }

    #[test]
    #[ignore = "diagnostic: probes the real Media Foundation H.264 encoder on this machine"]
    fn diag_probe_h264_encoder() {
        ensure_mf().expect("MFStartup");

        unsafe {
            // Empirically confirm how the windows-0.61 bindings map
            // IMFMediaEvent::GetType (the metadata declares the event type as
            // u32 "known value" rather than the raw GUID).
            for (label, typ) in [
                ("NeedInput", METransformNeedInput),
                ("HaveOutput", METransformHaveOutput),
            ] {
                if let Ok(ev) = MFCreateMediaEvent(
                    typ.0 as u32,
                    &windows::core::GUID::zeroed(),
                    windows::core::HRESULT(0),
                    None,
                ) {
                    let got = ev.GetType();
                    let ext = ev.GetExtendedType();
                    eprintln!(
                        "[diag] MFCreateMediaEvent({label}) GetType={got:?} ({typ:?}) GetExtendedType={ext:?}"
                    );
                } else {
                    eprintln!("[diag] MFCreateMediaEvent({label}) FAILED");
                }
            }

            let input_type = MFT_REGISTER_TYPE_INFO {
                guidMajorType: MFMediaType_Video,
                guidSubtype: MFVideoFormat_NV12,
            };
            let output_type = MFT_REGISTER_TYPE_INFO {
                guidMajorType: MFMediaType_Video,
                guidSubtype: MFVideoFormat_H264,
            };
            let mut activates: *mut Option<IMFActivate> = std::ptr::null_mut();
            let mut count: u32 = 0;
            let result = MFTEnumEx(
                MFT_CATEGORY_VIDEO_ENCODER,
                MFT_ENUM_FLAG_HARDWARE | MFT_ENUM_FLAG_SORTANDFILTER,
                Some(&input_type),
                Some(&output_type),
                &mut activates,
                &mut count,
            );
            if result.is_ok() && count > 0 && !activates.is_null() {
                let slice = std::slice::from_raw_parts(activates, count as usize);
                for (i, activate) in slice.iter().enumerate() {
                    if let Some(activate) = activate {
                        let name = {
                            let mut buf = vec![0u16; 256];
                            let mut len: u32 = 0;
                            if activate
                                .GetString(&MFT_FRIENDLY_NAME_Attribute, &mut buf, Some(&mut len))
                                .is_ok()
                            {
                                String::from_utf16_lossy(&buf[..(len as usize).min(buf.len())])
                            } else {
                                "?".into()
                            }
                        };
                        let flags = activate
                            .GetUINT32(&MF_TRANSFORM_FLAGS_Attribute)
                            .unwrap_or(0);
                        let is_async = activate.GetUINT32(&MF_TRANSFORM_ASYNC).unwrap_or(0) != 0;
                        eprintln!(
                            "[diag] hardware MFT #{i}: \"{name}\" flags=0x{flags:x} async={is_async}"
                        );
                        if i == 0 {
                            if let Ok(t) = activate.ActivateObject::<IMFTransform>() {
                                let mut input_info = MFT_INPUT_STREAM_INFO::default();
                                if t.GetInputStreamInfo(0, &mut input_info).is_ok() {
                                    eprintln!(
                                        "[diag]   input flags=0x{:x} cbSize={} cbAlignment={} cbMaxLookahead={}",
                                        input_info.dwFlags, input_info.cbSize, input_info.cbAlignment, input_info.cbMaxLookahead
                                    );
                                }
                                if let Ok(output_info) = t.GetOutputStreamInfo(0) {
                                    eprintln!(
                                        "[diag]   output flags=0x{:x} cbSize={} cbAlignment={}",
                                        output_info.dwFlags,
                                        output_info.cbSize,
                                        output_info.cbAlignment
                                    );
                                }
                                if let Ok(attrs) = t.GetAttributes() {
                                    for (label, key) in [
                                        ("ASYNC", &MF_TRANSFORM_ASYNC),
                                        ("ASYNC_UNLOCK", &MF_TRANSFORM_ASYNC_UNLOCK),
                                        ("FLAGS", &MF_TRANSFORM_FLAGS_Attribute),
                                    ] {
                                        let v = attrs.GetUINT32(key).ok();
                                        eprintln!("[diag]   attr {label}={v:?}");
                                    }
                                }
                            }
                        }
                    }
                }
                CoTaskMemFree(Some(activates as *const _));
            } else {
                eprintln!("[diag] MFTEnumEx(HARDWARE) returned no H.264 encoders");
            }
        }

        // The MS software encoder path (no D3D device manager required).
        for profile in [Some(66u32), Some(77u32), Some(100u32), None] {
            let label = match profile {
                Some(66) => "Baseline",
                Some(77) => "Main",
                Some(100) => "High",
                _ => "unset",
            };
            eprintln!("\n[diag] MS software encoder, profile={label} ({profile:?})");
            let transform = match create_ms_software_encoder() {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("[diag] software CoCreateInstance FAILED: {e}");
                    continue;
                }
            };
            let mut enc = match MfH264Encoder::from_transform(
                transform, 1920, 1080, 60, 8000, 60, profile, None,
            ) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("[diag] software negotiation FAILED: {e}");
                    continue;
                }
            };
            eprintln!("[diag] software negotiation OK");
            let nv12 = vec![0u8; nv12_packed_size(1920, 1080)];
            let mut total = 0;
            let mut saw_sync = false;
            for i in 0..100u32 {
                match enc.encode_frame(&nv12, std::time::Instant::now()) {
                    Ok(packets) => {
                        total += packets.len();
                        saw_sync |= packets.iter().any(|p| p.is_sync);
                    }
                    Err(e) => {
                        eprintln!("[diag] frame {i} encode FAILED: {e}");
                        break;
                    }
                }
            }
            eprintln!(
                "[diag] 100 frames -> {total} packets, saw_sync={saw_sync}, sps_pps_ready={}",
                enc.sps_pps_ready()
            );
        }

        // Test each enumerated hardware MFT individually through the production
        // negotiation path (D3D manager + memory buffers), to see which one the
        // encoder can actually drive.
        unsafe {
            let input_type = MFT_REGISTER_TYPE_INFO {
                guidMajorType: MFMediaType_Video,
                guidSubtype: MFVideoFormat_NV12,
            };
            let output_type = MFT_REGISTER_TYPE_INFO {
                guidMajorType: MFMediaType_Video,
                guidSubtype: MFVideoFormat_H264,
            };
            let mut activates: *mut Option<IMFActivate> = std::ptr::null_mut();
            let mut count: u32 = 0;
            let result = MFTEnumEx(
                MFT_CATEGORY_VIDEO_ENCODER,
                MFT_ENUM_FLAG_HARDWARE | MFT_ENUM_FLAG_SORTANDFILTER,
                Some(&input_type),
                Some(&output_type),
                &mut activates,
                &mut count,
            );
            if result.is_ok() && count > 0 && !activates.is_null() {
                let slice = std::slice::from_raw_parts(activates, count as usize);
                for (i, activate) in slice.iter().enumerate() {
                    if let Some(activate) = activate {
                        let name = mft_friendly_name(activate);
                        let is_async = activate.GetUINT32(&MF_TRANSFORM_ASYNC).unwrap_or(0) != 0;
                        let adapter_luid = activate.GetUINT64(&MFT_ENUM_ADAPTER_LUID).ok();
                        eprintln!(
                            "\n[diag] MFT #{i} \"{name}\" async={is_async} luid={adapter_luid:?} — production negotiation:"
                        );
                        let Ok(transform) = activate.ActivateObject::<IMFTransform>() else {
                            eprintln!("[diag]   activate FAILED");
                            continue;
                        };
                        let mut any_ok = false;
                        for profile in [Some(66u32), Some(77u32), Some(100u32), None] {
                            match MfH264Encoder::from_transform(
                                transform.clone(),
                                1920,
                                1080,
                                60,
                                8000,
                                60,
                                profile,
                                adapter_luid,
                            ) {
                                Ok(mut enc) => {
                                    any_ok = true;
                                    let nv12 = vec![0u8; nv12_packed_size(1920, 1080)];
                                    let mut total = 0;
                                    let mut saw_sync = false;
                                    let pace = diag_pace();
                                    for f in 0..100u32 {
                                        match enc.encode_frame(&nv12, std::time::Instant::now()) {
                                            Ok(packets) => {
                                                total += packets.len();
                                                saw_sync |= packets.iter().any(|p| p.is_sync);
                                            }
                                            Err(e) => {
                                                eprintln!("[diag]   frame {f} encode FAILED: {e}");
                                                break;
                                            }
                                        }
                                        if let Some(ms) = pace {
                                            std::thread::sleep(std::time::Duration::from_millis(
                                                ms,
                                            ));
                                        }
                                    }
                                    eprintln!(
                                        "[diag]   profile={profile:?} OK: 100 frames -> {total} packets, saw_sync={saw_sync} stats={:?}",
                                        enc.diag_async_stats()
                                    );
                                    break;
                                }
                                Err(e) => {
                                    eprintln!("[diag]   profile={profile:?} FAIL: {e}");
                                }
                            }
                        }
                        if !any_ok {
                            eprintln!("[diag]   no profile negotiated");
                        }
                    }
                }
                CoTaskMemFree(Some(activates as *const _));
            }
        }

        // Full production path: enumerate_h264_encoders (includes async MFTs,
        // driven via the event-based path) + with_profile (D3D manager +
        // profile iteration + software fallback).
        eprintln!("\n[diag] production path via MfH264Encoder::new(...):");
        let mut enc = match MfH264Encoder::new(1920, 1080, 60, 8000, 60) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("[diag] production init FAILED: {e}");
                return;
            }
        };
        let nv12 = vec![0u8; nv12_packed_size(1920, 1080)];
        let mut total = 0;
        let mut saw_sync = false;
        let pace = diag_pace();
        for i in 0..100u32 {
            match enc.encode_frame(&nv12, std::time::Instant::now()) {
                Ok(packets) => {
                    total += packets.len();
                    saw_sync |= packets.iter().any(|p| p.is_sync);
                }
                Err(e) => {
                    eprintln!("[diag] production frame {i} encode FAILED: {e}");
                    break;
                }
            }
            if let Some(ms) = pace {
                std::thread::sleep(std::time::Duration::from_millis(ms));
            }
        }
        eprintln!(
            "[diag] production 100 frames -> {total} packets, saw_sync={saw_sync}, sps_pps_ready={}, stats={:?}",
            enc.sps_pps_ready(),
            enc.diag_async_stats()
        );
    }
}

/// Optional pacing for the diag encode loops (`PRISM_DIAG_PACED_MS`). Feeding
/// frames at a real-time rate is how we distinguish real encoder throughput
/// from the async MFT pacing itself on wall-clock / sample timestamps.
///
/// Defaults to 4 ms (~250 fps, far above real-time) when the variable is unset.
/// A truly uncapped tight loop can saturate the GPU command queue — async
/// `UpdateSubresource`/`Flush` then blocks, which stalls the test thread.
#[cfg(test)]
fn diag_pace() -> Option<u64> {
    match std::env::var("PRISM_DIAG_PACED_MS") {
        Ok(v) => v.parse::<u64>().ok().filter(|ms| *ms > 0),
        Err(_) => Some(4),
    }
}

/// Best-effort friendly name of an enumerated MFT.
fn mft_friendly_name(activate: &IMFActivate) -> String {
    unsafe {
        let mut buf = vec![0u16; 256];
        let mut len: u32 = 0;
        if activate
            .GetString(&MFT_FRIENDLY_NAME_Attribute, &mut buf, Some(&mut len))
            .is_ok()
        {
            String::from_utf16_lossy(&buf[..(len as usize).min(buf.len())])
        } else {
            "?".into()
        }
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
            // Async MFTs want a clean shutdown: END_OF_STREAM then drain
            // remaining output, before the final END_STREAMING.
            if matches!(self.processing, Processing::Async(_)) {
                self.transform
                    .ProcessMessage(MFT_MESSAGE_NOTIFY_END_OF_STREAM, 0)
                    .ok();
            }
            self.transform
                .ProcessMessage(MFT_MESSAGE_NOTIFY_END_STREAMING, 0)
                .ok();
            self.transform
                .ProcessMessage(MFT_MESSAGE_COMMAND_FLUSH, 0)
                .ok();
        }
    }
}
