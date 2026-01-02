#[cfg(target_os = "windows")]
mod win32;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

use std::time::Instant;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CaptureError>;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("Platform not supported")]
    UnsupportedPlatform,

    #[error("Initialization failed: {0}")]
    InitFailed(String),

    #[error("Capture failed: {0}")]
    CaptureFailed(String),

    #[error("No frame available")]
    NoFrame,

    #[error("Timeout waiting for frame")]
    Timeout,

    #[error("Monitor not found")]
    MonitorNotFound,

    #[cfg(target_os = "windows")]
    #[error("Windows API Error: {0}")]
    WindowsError(#[from] windows::core::Error),
}


#[derive(Debug, Clone)]
pub enum CaptureMode {
    Monitor(u32),
    Window(String),
    Unknown,
}

#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub mode: CaptureMode,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            mode: CaptureMode::Monitor(0),
            width: 1920,
            height: 1080,
            fps: 60,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CapturedFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: PixelFormat,
    pub timestamp: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    BGRA,
    RGBA,
    NV12,
}

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
    pub is_primary: bool,
}

/// Trait that all platform-specific capture implementations must provide
pub trait ScreenCapture: Send + Sync {
    fn init(&mut self, config: CaptureConfig) -> Result<()>;
    fn capture_frame(&mut self) -> Result<CapturedFrame>;
    fn get_monitors(&self) -> Result<Vec<MonitorInfo>>;
    fn shutdown(&mut self) -> Result<()>;
}

// Select the correct platform implementation at compile time
#[cfg(target_os = "windows")]
type PlatformCapture = win32::DirectXCapture;

#[cfg(target_os = "linux")]
type PlatformCapture = linux::X11Capture;

#[cfg(target_os = "macos")]
type PlatformCapture = macos::CoreGraphicsCapture;

/// Main screen capture manager - automatically uses the correct platform
pub struct ScreenCaptureManager {
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    inner: PlatformCapture,
}

impl ScreenCaptureManager {
    pub fn new() -> Result<Self> {
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        return Err(CaptureError::UnsupportedPlatform);

        #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
        {
            tracing::info!("Creating screen capture manager for platform");
            Ok(Self {
                inner: PlatformCapture::default(),
            })
        }
    }

    pub fn init(&mut self, config: CaptureConfig) -> Result<()> {
        self.inner.init(config)
    }

    pub fn capture_frame(&mut self) -> Result<CapturedFrame> {
        self.inner.capture_frame()
    }

    pub fn get_monitors(&self) -> Result<Vec<MonitorInfo>> {
        self.inner.get_monitors()
    }

    pub fn shutdown(&mut self) -> Result<()> {
        self.inner.shutdown()
    }
}
