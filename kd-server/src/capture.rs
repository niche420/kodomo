#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "macos")]
mod mac;

#[derive(Clone)]
pub(crate) enum PixelFormat
{
    Bgra
}

pub struct Frame
{
    format: PixelFormat,
    data: Vec<u8>,
    width: u32,
    height: u32
}

impl Frame
{
    pub fn format(&self) -> PixelFormat
    {
        self.format.clone()
    }

    pub fn data(&self) -> &[u8]
    {
        &self.data
    }
}

pub trait FrameCapturer {
    fn capture_frame(&mut self) -> anyhow::Result<Frame>;
}

pub fn create_capturer() -> anyhow::Result<Box<dyn FrameCapturer>> {
    #[cfg(target_os = "windows")]
    return Ok(Box::new(windows::DxgiCapturer::new()?));
    #[cfg(target_os = "macos")]
    return Ok(Box::new(mac::ScreenCaptureKitCapturer::new()?));
}