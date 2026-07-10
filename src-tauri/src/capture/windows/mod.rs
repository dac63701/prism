use std::sync::Arc;
use std::time::Instant;

use windows::core::Interface;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;

use crate::capture::{
    CaptureBackend, CaptureConfig, CaptureError, CaptureSources, CapturedFrame, DisplayInfo,
    LatestFrame, PixelFormat,
};

pub struct WindowsCaptureBackend {
    device: Option<ID3D11Device>,
    context: Option<ID3D11DeviceContext>,
    duplication: Option<IDXGIOutputDuplication>,
    staging: Option<ID3D11Texture2D>,
    latest_frame: LatestFrame,
    active: bool,
    config: Option<CaptureConfig>,
    current_width: u32,
    current_height: u32,
}

impl WindowsCaptureBackend {
    pub fn new() -> Self {
        let (device, context) = create_d3d11_device().unwrap_or((None, None));
        Self {
            device,
            context,
            duplication: None,
            staging: None,
            latest_frame: LatestFrame::new(),
            active: false,
            config: None,
            current_width: 0,
            current_height: 0,
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
        let dst_stride = width * 4;
        let total_size = (dst_stride * height) as usize;
        let mut data: Vec<u8> = Vec::with_capacity(total_size);

        unsafe {
            let src_ptr = mapped.pData as *const u8;
            if src_stride == dst_stride {
                // Fast path: contiguous rows, single memcpy
                std::ptr::copy_nonoverlapping(src_ptr, data.as_mut_ptr(), total_size);
                data.set_len(total_size);
            } else {
                // Slow path: row pitch differs (e.g. GPU padding)
                for y in 0..height {
                    let src_offset = (y * src_stride) as usize;
                    let dst_offset = (y * dst_stride) as usize;
                    std::ptr::copy_nonoverlapping(
                        src_ptr.add(src_offset),
                        data.as_mut_ptr().add(dst_offset),
                        dst_stride as usize,
                    );
                }
                data.set_len(total_size);
            }
        }

        unsafe { context.Unmap(staging, 0) };

        unsafe { duplication.ReleaseFrame() }
            .map_err(|e| CaptureError::StreamError(format!("ReleaseFrame failed: {e}")))?;

        // Convert BGRA → NV12 for memory-efficient ring-buffer storage.
        // NV12 is 1.5 B/px (vs 4 B/px for BGRA), giving ~2.7× memory savings.
        let nv12_data = crate::capture::bgra_to_nv12(&data, width, height, dst_stride);

        let frame = CapturedFrame {
            data: Arc::new(nv12_data),
            width,
            height,
            stride: width, // NV12 Y-plane stride = width (tightly packed)
            pixel_format: PixelFormat::Nv12,
            timestamp: Instant::now(),
        };

        Ok(Some(frame))
    }
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

        self.duplication = Some(duplication);
        self.active = true;
        self.config = Some(config);

        Ok(())
    }

    fn stop(&mut self) -> Result<(), CaptureError> {
        self.duplication = None;
        self.staging = None;
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
