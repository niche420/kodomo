use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Gdi::{MonitorFromPoint, HMONITOR, MONITOR_DEFAULTTONULL};
use crate::{CaptureError, Result};

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub struct Monitor(HMONITOR);

unsafe impl Send for Monitor {}

impl Monitor {
    #[inline]
    pub fn primary() -> Result<Self> {
        let point = POINT { x: 0, y: 0 };
        let monitor = unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONULL) };

        if monitor.is_invalid() {
            return Err(CaptureError::MonitorNotFound);
        }

        Ok(Self(monitor))
    }
}