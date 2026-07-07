#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "macos")]
mod mac;
pub mod worker;

use std::path::PathBuf;

#[derive(Clone)]
pub(crate) enum PixelFormat {
    Bgra,
}

pub struct Frame {
    pub(crate) format: PixelFormat,
    pub(crate) data: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl Frame {
    pub fn format(&self) -> PixelFormat { self.format.clone() }
    pub fn data(&self) -> &[u8] { &self.data }
}

pub trait FrameCapturer {
    fn capture_frame(&mut self) -> Option<Frame>;
}

pub fn create_capturer(exe_path: &PathBuf) -> anyhow::Result<Box<dyn FrameCapturer>> {
    #[cfg(target_os = "windows")]
    return Ok(Box::new(windows::WindowsGraphicsCapturer::new(exe_path)?));
    #[cfg(target_os = "macos")]
    return Ok(Box::new(mac::ScreenCaptureKitCapturer::new()?));
}