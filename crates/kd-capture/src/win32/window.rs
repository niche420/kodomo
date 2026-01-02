use windows::core::HSTRING;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
use crate::{CaptureError, Result};
pub use windows::Graphics::Capture::GraphicsCaptureItem;
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub struct Window(HWND);

unsafe impl Send for Window {}

impl Window {
    pub fn find_by_name(name: &str) -> Result<Self> {
        let title = HSTRING::from(name);
        let window = unsafe { FindWindowW(None, &title)? };

        if window.is_invalid() {
            return Err(CaptureError::CaptureFailed("Window not found".into()));
        }

        Ok(Self(window))
    }

    #[inline]
    pub const fn as_ptr(&self) -> *mut std::ffi::c_void {
        self.0.0
    }
}

impl TryInto<GraphicsCaptureItem> for Window {
    type Error = CaptureError;
    #[inline]
    fn try_into(self) -> Result<GraphicsCaptureItem> {
        let window = self.0.clone();
        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        let item = unsafe { interop.CreateForWindow(window)? };
        Ok(item)
    }
}
