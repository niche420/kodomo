use std::ptr;
use windows::core::{BOOL, HSTRING};
use windows::Win32::Foundation::{HWND, LPARAM, RECT, TRUE};
use windows::Win32::UI::WindowsAndMessaging::{EnumChildWindows, FindWindowW, GetClientRect, GetDesktopWindow, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible, GWL_EXSTYLE, GWL_STYLE, WS_CHILD, WS_EX_TOOLWINDOW};
use crate::{CaptureError, Result};
pub use windows::Graphics::Capture::GraphicsCaptureItem;
use windows::Win32::System::Threading::GetCurrentProcessId;
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
    pub fn from_contains_name(title: &str) -> Result<Self> {
        let windows = Self::enumerate()?;

        let mut target_window = None;
        for window in windows {
            if window.title()?.contains(title) {
                target_window = Some(window);
                break;
            }
        }

        target_window.map_or_else(|| Err(CaptureError::WindowNotFound), Ok)
    }
    
    pub const fn from_ptr(hwnd: *mut std::ffi::c_void) -> Self {
        Self(HWND(hwnd))
    }

    #[inline]
    pub const fn as_ptr(&self) -> *mut std::ffi::c_void {
        self.0.0
    }

    #[inline]
    pub fn title(&self) -> Result<String> {
        let len = unsafe { GetWindowTextLengthW(self.0) };

        if len == 0 {
            return Ok(String::new());
        }

        let mut buf = vec![0u16; usize::try_from(len).unwrap() + 1];
        let copied = unsafe { GetWindowTextW(self.0, &mut buf) };
        if copied == 0 {
            return Ok(String::new());
        }

        let name = String::from_utf16(&buf[..copied as usize])
            .map_err(|_| CaptureError::PlatformError("String conv failed".into()))?;

        Ok(name)
    }

    #[inline]
    #[must_use]
    pub fn is_valid(&self) -> bool {
        if !unsafe { IsWindowVisible(self.0).as_bool() } {
            return false;
        }

        let mut id = 0;
        unsafe { GetWindowThreadProcessId(self.0, Some(&mut id)) };
        if id == unsafe { GetCurrentProcessId() } {
            return false;
        }

        let mut rect = RECT::default();
        let result = unsafe { GetClientRect(self.0, &mut rect) };
        if result.is_ok() {
            #[cfg(target_pointer_width = "64")]
            let styles = unsafe { GetWindowLongPtrW(self.0, GWL_STYLE) };
            #[cfg(target_pointer_width = "64")]
            let ex_styles = unsafe { GetWindowLongPtrW(self.0, GWL_EXSTYLE) };

            #[cfg(target_pointer_width = "32")]
            let styles = unsafe { GetWindowLongPtrW(self.window, GWL_STYLE) as isize };
            #[cfg(target_pointer_width = "32")]
            let ex_styles = unsafe { GetWindowLongPtrW(self.window, GWL_EXSTYLE) as isize };

            if (ex_styles & isize::try_from(WS_EX_TOOLWINDOW.0).unwrap()) != 0 {
                return false;
            }
            if (styles & isize::try_from(WS_CHILD.0).unwrap()) != 0 {
                return false;
            }
        } else {
            return false;
        }

        true
    }

    #[inline]
    pub fn enumerate() -> Result<Vec<Self>> {
        let mut windows: Vec<Self> = Vec::new();

        unsafe {
            EnumChildWindows(
                Some(GetDesktopWindow()),
                Some(Self::enum_windows_callback),
                LPARAM(ptr::addr_of_mut!(windows) as isize),
            )
                .ok()?;
        };

        Ok(windows)
    }

    // Callback used for enumerating all valid windows.
    #[inline]
    unsafe extern "system" fn enum_windows_callback(window: HWND, vec: LPARAM) -> BOOL {
        let windows = unsafe { &mut *(vec.0 as *mut Vec<Self>) };

        if Self::from_ptr(window.0).is_valid() {
            windows.push(Self(window));
        }

        TRUE
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
