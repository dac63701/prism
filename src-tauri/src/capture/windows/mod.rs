use crate::capture::{CaptureBackend, CaptureConfig, CaptureError, CapturedFrame};

pub struct WindowsCaptureBackend;

impl WindowsCaptureBackend {
    pub fn new() -> Self {
        Self
    }
}

impl CaptureBackend for WindowsCaptureBackend {
    fn start(&mut self, _config: CaptureConfig) -> Result<(), CaptureError> {
        Err(CaptureError::UnsupportedPlatform)
    }

    fn stop(&mut self) -> Result<(), CaptureError> {
        Err(CaptureError::UnsupportedPlatform)
    }

    fn read_latest_frame(&mut self) -> Option<CapturedFrame> {
        None
    }

    fn is_active(&self) -> bool {
        false
    }
}
