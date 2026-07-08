//! macOS screen capture backend using Apple's ScreenCaptureKit framework.
//!
//! Uses the [`screencapturekit`] crate (v8), which provides safe bindings to
//! SCStream / SCContentFilter. Requires macOS 12.3+ and Screen Recording
//! permission.

use std::sync::Arc;

use screencapturekit::cm::{CMSampleBuffer, CMSampleBufferExt};
use screencapturekit::shareable_content::SCShareableContent;
use screencapturekit::stream::configuration::pixel_format::PixelFormat as ScPixelFormat;
use screencapturekit::stream::configuration::SCStreamConfiguration;
use screencapturekit::stream::content_filter::SCContentFilter;
use screencapturekit::stream::output_type::SCStreamOutputType;
use screencapturekit::stream::sc_stream::SCStream;

use crate::capture::{
    AppInfo, CaptureBackend, CaptureConfig, CaptureError, CaptureSources, CaptureTarget,
    CapturedFrame, DisplayInfo, LatestFrame, PixelFormat,
};

/// macOS capture backend backed by ScreenCaptureKit.
pub struct MacCaptureBackend {
    stream: Option<SCStream>,
    latest: Arc<LatestFrame>,
    active: bool,
}

impl MacCaptureBackend {
    pub fn new() -> Self {
        Self {
            stream: None,
            latest: Arc::new(LatestFrame::new()),
            active: false,
        }
    }
}

impl CaptureBackend for MacCaptureBackend {
    fn start(&mut self, config: CaptureConfig) -> Result<(), CaptureError> {
        if self.active {
            return Ok(());
        }

        // --- 1. Enumerate shareable content --------------------------------
        let content = SCShareableContent::get()
            .map_err(|e| CaptureError::StartFailed(format!("SCShareableContent::get: {e}")))?;

        // --- 2. Build content filter from target ---------------------------
        let (filter, width, height) = build_filter(&config.target, &content)?;

        // --- 3. Stream configuration ---------------------------------------
        let mut cfg = SCStreamConfiguration::new()
            .with_width(width)
            .with_height(height);
        cfg.set_pixel_format(ScPixelFormat::BGRA);
        cfg.set_shows_cursor(config.capture_cursor);

        // --- 4. Create stream ----------------------------------------------
        let mut stream = SCStream::new(&filter, &cfg);

        // --- 5. Register output handler ------------------------------------
        let latest = self.latest.clone();
        let registered = stream.add_output_handler(
            move |sample: CMSampleBuffer, of_type: SCStreamOutputType| {
                if of_type != SCStreamOutputType::Screen {
                    return;
                }
                if let Some(pixel_buffer) = sample.image_buffer() {
                    Self::handle_frame(&latest, &pixel_buffer);
                }
            },
            SCStreamOutputType::Screen,
        );

        if registered.is_none() {
            return Err(CaptureError::StartFailed(
                "ScreenCaptureKit refused to register the output handler".into(),
            ));
        }

        stream
            .start_capture()
            .map_err(|e| CaptureError::StartFailed(format!("start_capture: {e}")))?;

        self.stream = Some(stream);
        self.active = true;
        tracing::info!(
            "macOS capture started (target={:?}, {}x{})",
            config.target,
            width,
            height
        );
        Ok(())
    }

    fn stop(&mut self) -> Result<(), CaptureError> {
        if let Some(stream) = self.stream.take() {
            stream
                .stop_capture()
                .map_err(|e| CaptureError::StreamError(format!("stop_capture: {e}")))?;
        }
        self.active = false;
        tracing::info!("macOS capture stopped");
        Ok(())
    }

    fn read_latest_frame(&mut self) -> Option<CapturedFrame> {
        self.latest.take()
    }

    fn is_active(&self) -> bool {
        self.active
    }
}

// ---------------------------------------------------------------------------
// Content filter builder
// ---------------------------------------------------------------------------

type FilterResult = Result<(SCContentFilter, u32, u32), CaptureError>;

fn build_filter(target: &CaptureTarget, content: &SCShareableContent) -> FilterResult {
    match target {
        CaptureTarget::Display | CaptureTarget::DisplayId(_) => {
            build_display_filter(target, content)
        }
        CaptureTarget::Window(window_id) => build_window_filter(*window_id, content),
        CaptureTarget::Application(bundle_id) => build_application_filter(bundle_id, content),
    }
}

fn build_display_filter(target: &CaptureTarget, content: &SCShareableContent) -> FilterResult {
    let displays = content.displays();
    let sc_display = match target {
        CaptureTarget::DisplayId(id) => displays
            .iter()
            .find(|d| d.display_id() == *id)
            .ok_or_else(|| CaptureError::StartFailed(format!("Display ID {id} not found")))?,
        _ => displays
            .first()
            .ok_or_else(|| CaptureError::StartFailed("No display found".into()))?,
    };

    let w = sc_display.width();
    let h = sc_display.height();
    let filter = SCContentFilter::create()
        .with_display(sc_display)
        .with_excluding_windows(&[])
        .build();

    Ok((filter, w, h))
}

fn build_window_filter(window_id: u32, content: &SCShareableContent) -> FilterResult {
    let windows = content.windows();
    let sc_window = windows
        .iter()
        .find(|w| w.window_id() == window_id)
        .ok_or_else(|| CaptureError::StartFailed(format!("Window ID {window_id} not found")))?;

    let filter = SCContentFilter::create().with_window(sc_window).build();
    let f = sc_window.frame();
    Ok((filter, f.size.width as u32, f.size.height as u32))
}

fn build_application_filter(bundle_id: &str, content: &SCShareableContent) -> FilterResult {
    let applications = content.applications();
    let app = applications
        .iter()
        .find(|a| a.bundle_identifier() == bundle_id)
        .ok_or_else(|| CaptureError::StartFailed(format!("Application {bundle_id} not found")))?;

    let displays = content.displays();
    let sc_display = displays
        .first()
        .ok_or_else(|| CaptureError::StartFailed("No display found".into()))?;

    let w = sc_display.width();
    let h = sc_display.height();
    let filter = SCContentFilter::create()
        .with_display(sc_display)
        .with_including_applications(&[app], &[])
        .build();

    Ok((filter, w, h))
}

// ---------------------------------------------------------------------------
// Source enumeration (for the UI source selector)
// ---------------------------------------------------------------------------

pub fn enumerate_sources() -> CaptureSources {
    let content = match SCShareableContent::create()
        .with_on_screen_windows_only(true)
        .with_exclude_desktop_windows(true)
        .get()
    {
        Ok(c) => c,
        Err(_) => {
            return CaptureSources {
                displays: vec![],
                applications: vec![],
            };
        }
    };

    let mut displays: Vec<DisplayInfo> = content
        .displays()
        .iter()
        .map(|d| DisplayInfo {
            display_id: d.display_id(),
            width: d.width(),
            height: d.height(),
            is_main: false,
        })
        .collect();
    if let Some(first) = displays.first_mut() {
        first.is_main = true;
    }

    // Deduplicate applications by bundle_id
    let mut seen: Vec<AppInfo> = Vec::new();
    for window in content.windows() {
        if !window.is_on_screen() {
            continue;
        }
        if let Some(app) = window.owning_application() {
            let bid = app.bundle_identifier();
            if let Some(entry) = seen.iter_mut().find(|a: &&mut AppInfo| a.bundle_id == bid) {
                entry.window_count += 1;
            } else {
                seen.push(AppInfo {
                    pid: app.process_id(),
                    name: app.application_name(),
                    bundle_id: bid,
                    window_count: 1,
                });
            }
        }
    }

    CaptureSources {
        displays,
        applications: seen,
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

impl MacCaptureBackend {
    fn handle_frame(latest: &LatestFrame, pixel_buffer: &screencapturekit::cv::CVPixelBuffer) {
        let guard = match pixel_buffer.lock_read_only() {
            Ok(g) => g,
            Err(e) => {
                tracing::warn!("Failed to lock CVPixelBuffer (error {e})");
                return;
            }
        };

        let width = pixel_buffer.width() as u32;
        let height = pixel_buffer.height() as u32;
        let stride = pixel_buffer.bytes_per_row() as u32;

        let frame_data = guard.as_slice().to_vec();

        latest.store(CapturedFrame {
            data: Arc::new(frame_data),
            width,
            height,
            stride,
            pixel_format: PixelFormat::Bgra,
            timestamp: std::time::Instant::now(),
        });
    }
}
