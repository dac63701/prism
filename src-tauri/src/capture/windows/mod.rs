use std::sync::Arc;
use std::time::Instant;

use std::mem::ManuallyDrop;

use windows::core::Interface;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;

use crate::capture::{
    bgra_to_nv12, bgra_to_nv12_scaled, CaptureBackend, CaptureConfig, CaptureError, CaptureSources,
    CapturedFrame, DisplayInfo, LatestFrame, PixelFormat,
};

/// Force the CPU conversion path even when a D3D11 video processor is available
/// (diagnostics / driver workarounds).
fn force_cpu_capture() -> bool {
    std::env::var("PRISM_FORCE_CPU_CAPTURE").map(|v| v == "1").unwrap_or(false)
}

pub struct WindowsCaptureBackend {
    device: Option<ID3D11Device>,
    context: Option<ID3D11DeviceContext>,
    duplication: Option<IDXGIOutputDuplication>,
    /// BGRA staging texture for the CPU fallback path.
    staging: Option<ID3D11Texture2D>,
    /// GPU conversion pipeline (BGRA → NV12 + optional downscale).
    video_pipeline: Option<VideoPipeline>,
    #[allow(dead_code)]
    latest_frame: LatestFrame,
    active: bool,
    config: Option<CaptureConfig>,
    current_width: u32,
    current_height: u32,
    /// Requested output dimensions from settings (0 = native).
    target_width: u32,
    target_height: u32,
}

/// D3D11 video processor pipeline: converts the desktop texture (BGRA) to NV12
/// at the configured output resolution entirely on the GPU.
struct VideoPipeline {
    /// Held for the pipeline lifetime (views reference the enumerator/device).
    #[allow(dead_code)]
    video_device: ID3D11VideoDevice,
    #[allow(dead_code)]
    enumerator: ID3D11VideoProcessorEnumerator,
    processor: ID3D11VideoProcessor,
    /// Persistent BGRA copy of the desktop surface (VP input).
    input_texture: ID3D11Texture2D,
    input_view: ID3D11VideoProcessorInputView,
    /// NV12 output surface (VP target).
    output_texture: ID3D11Texture2D,
    output_view: ID3D11VideoProcessorOutputView,
    /// CPU-readable NV12 staging surface.
    staging: ID3D11Texture2D,
    #[allow(dead_code)]
    src_width: u32,
    #[allow(dead_code)]
    src_height: u32,
    dst_width: u32,
    dst_height: u32,
}

impl WindowsCaptureBackend {
    pub fn new() -> Self {
        let (device, context) = create_d3d11_device().unwrap_or((None, None));
        Self {
            device,
            context,
            duplication: None,
            staging: None,
            video_pipeline: None,
            latest_frame: LatestFrame::new(),
            active: false,
            config: None,
            current_width: 0,
            current_height: 0,
            target_width: 0,
            target_height: 0,
        }
    }

    fn find_output(&self, target: &CaptureConfig) -> Result<IDXGIOutput1, CaptureError> {
        let device = self
            .device
            .as_ref()
            .ok_or(CaptureError::UnsupportedPlatform)?;
        let dxgi_device: IDXGIDevice = device
            .cast()
            .map_err(|_| CaptureError::StartFailed("Failed to cast to IDXGIDevice".into()))?;
        let adapter = unsafe { dxgi_device.GetAdapter() }
            .map_err(|_| CaptureError::StartFailed("No DXGI adapter".into()))?;

        let target_display_id = match &target.target {
            crate::capture::CaptureTarget::DisplayId(id) => *id,
            _ => 0,
        };

        let mut output_index = 0u32;
        loop {
            match unsafe { adapter.EnumOutputs(output_index) } {
                Ok(output) => {
                    let output1: IDXGIOutput1 = output.cast().map_err(|_| {
                        CaptureError::StartFailed("Output doesn't support IDXGIOutput1".into())
                    })?;

                    if output_index == target_display_id {
                        return Ok(output1);
                    }
                }
                Err(_) => break,
            }
            output_index += 1;
        }

        Err(CaptureError::StartFailed("Target display not found".into()))
    }

    fn ensure_staging(
        device: &ID3D11Device,
        staging: &mut Option<ID3D11Texture2D>,
        width: u32,
        height: u32,
        format: DXGI_FORMAT,
    ) -> Result<(), CaptureError> {
        let needs_recreate = match staging.as_ref() {
            Some(existing) => {
                let mut desc = D3D11_TEXTURE2D_DESC::default();
                unsafe { existing.GetDesc(&mut desc) };
                desc.Width != width || desc.Height != height
            }
            None => true,
        };

        if needs_recreate {
            let desc = D3D11_TEXTURE2D_DESC {
                Width: width,
                Height: height,
                MipLevels: 1,
                ArraySize: 1,
                Format: format,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_STAGING,
                BindFlags: D3D11_BIND_FLAG(0).0 as u32,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                MiscFlags: D3D11_RESOURCE_MISC_FLAG(0).0 as u32,
            };
            let mut new_staging: Option<ID3D11Texture2D> = None;
            let hr = unsafe { device.CreateTexture2D(&desc, None, Some(&mut new_staging)) };
            if hr.is_err() {
                return Err(CaptureError::StartFailed(
                    "Failed to create staging texture".into(),
                ));
            }
            *staging = new_staging;
        }

        Ok(())
    }

    /// Try to create the GPU conversion pipeline. Returns `None` (with a log)
    /// when the driver/GPU doesn't support the BGRA→NV12 path, so callers fall
    /// back to the CPU conversion.
    fn create_video_pipeline(
        &self,
        src_width: u32,
        src_height: u32,
        src_format: DXGI_FORMAT,
        dst_width: u32,
        dst_height: u32,
    ) -> Option<VideoPipeline> {
        if force_cpu_capture() {
            eprintln!("[capture] PRISM_FORCE_CPU_CAPTURE=1 — using CPU conversion");
            return None;
        }
        let device = self.device.as_ref()?;
        let video_device: ID3D11VideoDevice = device.cast().ok()?;

        let desc = D3D11_VIDEO_PROCESSOR_CONTENT_DESC {
            InputFrameFormat: D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE,
            InputFrameRate: DXGI_RATIONAL {
                Numerator: 60,
                Denominator: 1,
            },
            InputWidth: src_width,
            InputHeight: src_height,
            OutputFrameRate: DXGI_RATIONAL {
                Numerator: 60,
                Denominator: 1,
            },
            OutputWidth: dst_width,
            OutputHeight: dst_height,
            Usage: D3D11_VIDEO_USAGE_PLAYBACK_NORMAL,
        };
        let enumerator = unsafe { video_device.CreateVideoProcessorEnumerator(&desc) }
            .map_err(|e| {
                eprintln!("[capture] CreateVideoProcessorEnumerator failed: {e}");
                e
            })
            .ok()?;

        // Verify the formats we need are supported. `CheckVideoProcessorFormat`
        // reports support for an input format; the enumerator1 variant checks
        // an explicit input→output conversion.
        let input_ok = unsafe { enumerator.CheckVideoProcessorFormat(src_format) }.is_ok();
        if !input_ok {
            eprintln!(
                "[capture] VP input format {src_format:?} unsupported — CPU fallback"
            );
            return None;
        }
        let out_ok = unsafe { enumerator.CheckVideoProcessorFormat(DXGI_FORMAT_NV12) }.is_ok()
            || enumerator
                .cast::<ID3D11VideoProcessorEnumerator1>()
                .ok()
                .is_some_and(|en1| {
                    unsafe {
                        en1.CheckVideoProcessorFormatConversion(
                            src_format,
                            DXGI_COLOR_SPACE_YCBCR_FULL_G22_NONE_P709_X601,
                            DXGI_FORMAT_NV12,
                            DXGI_COLOR_SPACE_YCBCR_FULL_G22_NONE_P709_X601,
                        )
                    }
                    .is_ok()
                });
        if !out_ok {
            eprintln!("[capture] VP NV12 output unsupported — CPU fallback");
            return None;
        }

        let processor = unsafe { video_device.CreateVideoProcessor(&enumerator, 0) }
            .map_err(|e| {
                eprintln!("[capture] CreateVideoProcessor failed: {e}");
                e
            })
            .ok()?;

        // Persistent input texture (BGRA at source size) with a reusable VP input view.
        let input_desc = D3D11_TEXTURE2D_DESC {
            Width: src_width,
            Height: src_height,
            MipLevels: 1,
            ArraySize: 1,
            Format: src_format,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32 | D3D11_BIND_RENDER_TARGET.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        let mut input_texture: Option<ID3D11Texture2D> = None;
        unsafe { device.CreateTexture2D(&input_desc, None, Some(&mut input_texture)) }
            .map_err(|e| {
                eprintln!("[capture] create VP input texture failed: {e}");
                e
            })
            .ok()?;
        let input_texture = input_texture?;

        let input_view_desc = D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC {
            FourCC: 0,
            ViewDimension: D3D11_VPIV_DIMENSION_TEXTURE2D,
            Anonymous: D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC_0 {
                Texture2D: D3D11_TEX2D_VPIV {
                    MipSlice: 0,
                    ArraySlice: 0,
                },
            },
        };
        let mut input_view: Option<ID3D11VideoProcessorInputView> = None;
        unsafe {
            video_device.CreateVideoProcessorInputView(
                &input_texture,
                &enumerator,
                &input_view_desc,
                Some(&mut input_view),
            )
        }
        .map_err(|e| {
            eprintln!("[capture] create VP input view failed: {e}");
            e
        })
        .ok()?;
        let input_view = input_view?;

        // NV12 output texture + VP output view + staging.
        let output_desc = D3D11_TEXTURE2D_DESC {
            Width: dst_width,
            Height: dst_height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_NV12,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_RENDER_TARGET.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: D3D11_RESOURCE_MISC_FLAG(0).0 as u32,
        };
        let mut output_texture: Option<ID3D11Texture2D> = None;
        unsafe { device.CreateTexture2D(&output_desc, None, Some(&mut output_texture)) }
            .map_err(|e| {
                eprintln!("[capture] create VP output texture failed: {e}");
                e
            })
            .ok()?;
        let output_texture = output_texture?;

        let output_view_desc = D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC {
            ViewDimension: D3D11_VPOV_DIMENSION_TEXTURE2D,
            Anonymous: D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC_0 {
                Texture2D: D3D11_TEX2D_VPOV { MipSlice: 0 },
            },
        };
        let mut output_view: Option<ID3D11VideoProcessorOutputView> = None;
        unsafe {
            video_device.CreateVideoProcessorOutputView(
                &output_texture,
                &enumerator,
                &output_view_desc,
                Some(&mut output_view),
            )
        }
        .map_err(|e| {
            eprintln!("[capture] create VP output view failed: {e}");
            e
        })
        .ok()?;
        let output_view = output_view?;

        // NV12 staging for CPU readback.
        let mut staging: Option<ID3D11Texture2D> = None;
        Self::ensure_staging(device, &mut staging, dst_width, dst_height, DXGI_FORMAT_NV12)
            .map_err(|e| {
                eprintln!("[capture] create VP staging failed: {e}");
                e
            })
            .ok()?;
        let staging = staging?;

        eprintln!(
            "[capture] GPU video processor active: {}x{} -> {}x{} (NV12)",
            src_width, src_height, dst_width, dst_height
        );

        Some(VideoPipeline {
            video_device,
            enumerator,
            processor,
            input_texture,
            input_view,
            output_texture,
            output_view,
            staging,
            src_width,
            src_height,
            dst_width,
            dst_height,
        })
    }

    /// Acquire + convert a frame through the GPU video processor pipeline.
    /// The desktop surface is copied to a persistent BGRA texture, converted to
    /// NV12 (and downscaled) by the VP, then read back through NV12 staging.
    fn acquire_frame_gpu_impl(
        duplication: &IDXGIOutputDuplication,
        context: &ID3D11DeviceContext,
        pipeline: &VideoPipeline,
        desktop_texture: &ID3D11Texture2D,
    ) -> Result<Option<CapturedFrame>, CaptureError> {
        // Every duplicated frame MUST be released before the next
        // AcquireNextFrame, or the duplication enters DXGI_ERROR_ACCESS_LOST
        // and capture dies permanently. The guard releases on every exit path,
        // including cast/VP errors that previously leaked the frame.
        struct ReleaseFrame<'a>(&'a IDXGIOutputDuplication);
        impl Drop for ReleaseFrame<'_> {
            fn drop(&mut self) {
                unsafe {
                    self.0.ReleaseFrame().ok();
                }
            }
        }
        let _release = ReleaseFrame(duplication);

        unsafe { context.CopyResource(&pipeline.input_texture, desktop_texture) };

        let mut stream = D3D11_VIDEO_PROCESSOR_STREAM {
            Enable: true.into(),
            OutputIndex: 0,
            InputFrameOrField: 0,
            PastFrames: 0,
            FutureFrames: 0,
            ppPastSurfaces: std::ptr::null_mut(),
            pInputSurface: ManuallyDrop::new(Some(pipeline.input_view.clone())),
            ppFutureSurfaces: std::ptr::null_mut(),
            ppPastSurfacesRight: std::ptr::null_mut(),
            pInputSurfaceRight: ManuallyDrop::new(None),
            ppFutureSurfacesRight: std::ptr::null_mut(),
        };

        let video_context: ID3D11VideoContext = context
            .cast()
            .map_err(|e| CaptureError::StreamError(format!("Failed to get video context: {e}")))?;

        let result = unsafe {
            video_context.VideoProcessorBlt(
                &pipeline.processor,
                &pipeline.output_view,
                0,
                core::slice::from_ref(&stream),
            )
        };
        unsafe { ManuallyDrop::drop(&mut stream.pInputSurface) };
        result.map_err(|e| CaptureError::StreamError(format!("VideoProcessorBlt failed: {e}")))?;

        unsafe { context.CopyResource(&pipeline.staging, &pipeline.output_texture) };

        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        let hr = unsafe { context.Map(&pipeline.staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)) };
        if hr.is_err() {
            return Err(CaptureError::StreamError(
                "Failed to map NV12 staging texture".into(),
            ));
        }

        let nv12 = pack_nv12_staging(mapped, pipeline.dst_width, pipeline.dst_height);

        unsafe { context.Unmap(&pipeline.staging, 0) };

        let frame = CapturedFrame {
            data: Arc::new(nv12),
            width: pipeline.dst_width,
            height: pipeline.dst_height,
            stride: pipeline.dst_width,
            pixel_format: PixelFormat::Nv12,
            timestamp: Instant::now(),
        };

        Ok(Some(frame))
    }

    fn acquire_frame(&mut self) -> Result<Option<CapturedFrame>, CaptureError> {
        let duplication = self.duplication.as_ref().ok_or(CaptureError::NoFrame)?;
        let context = self.context.as_ref().ok_or(CaptureError::NoFrame)?;

        let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
        let mut desktop_resource: Option<IDXGIResource> = None;

        let hr = unsafe {
            duplication.AcquireNextFrame(
                0,
                &mut frame_info,
                &mut desktop_resource as *mut Option<IDXGIResource>,
            )
        };

        if let Err(e) = hr {
            let code = e.code();
            if code == DXGI_ERROR_WAIT_TIMEOUT {
                return Ok(None);
            }
            if code == DXGI_ERROR_ACCESS_LOST || code == DXGI_ERROR_DEVICE_RESET {
                self.duplication = None;
                return Err(CaptureError::StreamError(
                    "Desktop duplication access lost".into(),
                ));
            }
            return Err(CaptureError::StreamError(format!(
                "AcquireNextFrame failed: {e}"
            )));
        }

        let resource = desktop_resource.ok_or(CaptureError::NoFrame)?;
        let src_texture: ID3D11Texture2D = resource
            .cast()
            .map_err(|e| CaptureError::StreamError(format!("Failed to cast resource: {e}")))?;

        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { src_texture.GetDesc(&mut desc) };
        let width = desc.Width;
        let height = desc.Height;
        let format = desc.Format;

        // GPU fast path.
        if let Some(pipeline) = self.video_pipeline.as_ref() {
            let result =
                Self::acquire_frame_gpu_impl(duplication, context, pipeline, &src_texture);
            drop(src_texture);
            return result;
        }

        // CPU fallback path.
        let device = self.device.as_ref().ok_or(CaptureError::NoFrame)?;
        Self::ensure_staging(device, &mut self.staging, width, height, format)?;
        let staging = self.staging.as_ref().ok_or(CaptureError::NoFrame)?;

        unsafe { context.CopyResource(staging, &src_texture) };

        drop(src_texture);

        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        let hr = unsafe { context.Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)) };

        if hr.is_err() {
            unsafe { duplication.ReleaseFrame() }.ok();
            return Err(CaptureError::StreamError(
                "Failed to map staging texture".into(),
            ));
        }

        let src_stride = mapped.RowPitch;
        let mapped_data = unsafe {
            std::slice::from_raw_parts(mapped.pData as *const u8, (src_stride * height) as usize)
        };

        let (dst_w, dst_h) = self.output_dimensions(width, height);
        let nv12_data = if dst_w == width && dst_h == height {
            bgra_to_nv12(mapped_data, width, height, src_stride)
        } else {
            bgra_to_nv12_scaled(mapped_data, width, height, src_stride, dst_w, dst_h)
        };

        unsafe { context.Unmap(staging, 0) };

        unsafe { duplication.ReleaseFrame() }
            .map_err(|e| CaptureError::StreamError(format!("ReleaseFrame failed: {e}")))?;

        let frame = CapturedFrame {
            data: Arc::new(nv12_data),
            width: dst_w,
            height: dst_h,
            stride: dst_w,
            pixel_format: PixelFormat::Nv12,
            timestamp: Instant::now(),
        };

        Ok(Some(frame))
    }

    /// Effective output size for this frame: configured target or native.
    fn output_dimensions(&self, src_width: u32, src_height: u32) -> (u32, u32) {
        if self.target_width > 0 && self.target_height > 0 {
            (self.target_width, self.target_height)
        } else {
            (src_width, src_height)
        }
    }
}

/// Copy NV12 data out of a mapped staging texture into a tightly-packed buffer
/// (Y plane followed by interleaved UV plane), de-padding rows when the GPU
/// row pitch is aligned beyond `width`.
fn pack_nv12_staging(mapped: D3D11_MAPPED_SUBRESOURCE, width: u32, height: u32) -> Vec<u8> {
    let y_size = (width * height) as usize;
    let uv_width = width.div_ceil(2);
    let uv_height = height.div_ceil(2);
    let uv_size = (uv_width * uv_height * 2) as usize;
    let total = y_size + uv_size;
    let mut out = Vec::with_capacity(total);
    out.resize(total, 0);

    let pitch = mapped.RowPitch as usize;
    let src = unsafe { std::slice::from_raw_parts(mapped.pData as *const u8, pitch * height as usize + pitch * uv_height as usize) };

    if pitch == width as usize {
        // Tightly packed fast path: single memcpy.
        out[..y_size].copy_from_slice(&src[..y_size]);
        out[y_size..].copy_from_slice(&src[y_size..y_size + uv_size]);
        return out;
    }

    // Row-wise de-pad.
    let y_plane = &mut out[..y_size];
    for y in 0..height as usize {
        let row_src = &src[y * pitch..y * pitch + width as usize];
        y_plane[y * width as usize..(y + 1) * width as usize].copy_from_slice(row_src);
    }
    let uv_plane = &mut out[y_size..];
    let uv_start = height as usize * pitch;
    for uv_row in 0..uv_height as usize {
        let row_src = &src[uv_start + uv_row * pitch..uv_start + uv_row * pitch + width as usize];
        let dst_off = uv_row * uv_width as usize * 2;
        uv_plane[dst_off..dst_off + width as usize].copy_from_slice(row_src);
    }

    out
}

impl CaptureBackend for WindowsCaptureBackend {
    fn start(&mut self, config: CaptureConfig) -> Result<(), CaptureError> {
        if self.device.is_none() {
            return Err(CaptureError::UnsupportedPlatform);
        }

        let output1 = self.find_output(&config)?;

        let device = self
            .device
            .as_ref()
            .ok_or(CaptureError::UnsupportedPlatform)?;
        let duplication = unsafe { output1.DuplicateOutput(device) }
            .map_err(|e| CaptureError::StartFailed(format!("DuplicateOutput failed: {e}")))?;

        let dup_desc = unsafe { duplication.GetDesc() };
        self.current_width = dup_desc.ModeDesc.Width;
        self.current_height = dup_desc.ModeDesc.Height;
        self.target_width = config.target_width;
        self.target_height = config.target_height;

        // Create the GPU conversion pipeline once. Any failure falls back to
        // the CPU conversion path transparently (old/weak GPUs, odd drivers).
        let (dst_w, dst_h) = self.output_dimensions(self.current_width, self.current_height);
        self.video_pipeline = self.create_video_pipeline(
            self.current_width,
            self.current_height,
            dup_desc.ModeDesc.Format,
            dst_w,
            dst_h,
        );

        self.duplication = Some(duplication);
        self.active = true;
        self.config = Some(config);

        Ok(())
    }

    fn stop(&mut self) -> Result<(), CaptureError> {
        self.duplication = None;
        self.staging = None;
        self.video_pipeline = None;
        self.active = false;
        self.config = None;
        Ok(())
    }

    fn read_latest_frame(&mut self) -> Option<CapturedFrame> {
        self.acquire_frame().unwrap_or_default()
    }

    fn is_active(&self) -> bool {
        self.active
    }
}

impl Drop for WindowsCaptureBackend {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

// ── D3D11 device creation ───────────────────────────────────────────────

fn create_d3d11_device() -> Result<(Option<ID3D11Device>, Option<ID3D11DeviceContext>), CaptureError>
{
    let mut device: Option<ID3D11Device> = None;
    let mut context: Option<ID3D11DeviceContext> = None;

    let hr = unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_FLAG(0),
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )
    };

    if let Err(e) = hr {
        return Err(CaptureError::StartFailed(format!(
            "D3D11CreateDevice failed: {}",
            e
        )));
    }

    Ok((device, context))
}

// ── Source enumeration ───────────────────────────────────────────────────

pub fn enumerate_sources() -> CaptureSources {
    let mut displays = Vec::new();

    if let Ok(factory) = create_dxgi_factory() {
        let mut adapter_index = 0u32;
        loop {
            match unsafe { factory.EnumAdapters1(adapter_index) } {
                Ok(adapter) => {
                    let mut output_index = 0u32;
                    loop {
                        match unsafe { adapter.EnumOutputs(output_index) } {
                            Ok(output) => {
                                if let Ok(desc) = unsafe { output.GetDesc() } {
                                    displays.push(DisplayInfo {
                                        display_id: output_index,
                                        width: desc.DesktopCoordinates.right as u32
                                            - desc.DesktopCoordinates.left as u32,
                                        height: desc.DesktopCoordinates.bottom as u32
                                            - desc.DesktopCoordinates.top as u32,
                                        is_main: output_index == 0,
                                        refresh_rate: primary_display_refresh_rate(),
                                    });
                                }
                            }
                            Err(_) => break,
                        }
                        output_index += 1;
                    }
                }
                Err(_) => break,
            }
            adapter_index += 1;
        }
    }

    CaptureSources {
        displays,
        applications: vec![],
    }
}

fn create_dxgi_factory() -> Result<IDXGIFactory1, CaptureError> {
    unsafe { CreateDXGIFactory1() }
        .map_err(|e| CaptureError::StartFailed(format!("CreateDXGIFactory1 failed: {e}")))
}

/// Refresh rate of the main display in Hz, or 0 if undetectable.
pub fn primary_display_refresh_rate() -> u32 {
    use windows::Win32::Graphics::Gdi::{EnumDisplaySettingsW, DEVMODEW, ENUM_CURRENT_SETTINGS};
    let mut devmode = DEVMODEW::default();
    let ok = unsafe { EnumDisplaySettingsW(None, ENUM_CURRENT_SETTINGS, &mut devmode) };
    if ok.as_bool() && devmode.dmDisplayFrequency > 0 {
        devmode.dmDisplayFrequency
    } else {
        0
    }
}