use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::info;
use crate::{CaptureConfig, CaptureError, CaptureHandler, CaptureMode, MonitorInfo, Result, ScreenCapture};

mod d3d11;
mod window;
mod frame_pool;
mod monitor;
mod dd;
mod gfx_capture;

pub struct DirectXCapture {
    should_stop: Arc<AtomicBool>,
}

impl DirectXCapture {
    pub fn new() -> Result<Self> {
        Ok(Self {
            should_stop: Arc::new(AtomicBool::new(false)),
        })
    }
}

impl ScreenCapture for DirectXCapture {
    fn start<H: CaptureHandler>(&mut self, config: CaptureConfig, handler: Arc<Mutex<H>>) -> Result<()> {
        self.should_stop.store(false, Ordering::Relaxed);

        info!("Starting DirectX capture: {:?}", config);

        match config.mode {
            CaptureMode::Window(name) => {
                gfx_capture::start_window_capture(name, &self.should_stop, handler)
            }
            CaptureMode::Monitor(idx) => {
                dd::start_monitor_capture(config, &self.should_stop, handler)
            }
            CaptureMode::Unknown => {
                Err(CaptureError::InitFailed("Unknown capture mode".into()))
            }
        }
    }

    fn stop(&self) -> Result<()> {
        info!("Stopping DirectX capture");
        self.should_stop.store(true, Ordering::Relaxed);

        // Post quit message to stop message loop
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::*;
            use windows::Win32::System::Threading::GetCurrentThreadId;
            let _ = PostThreadMessageW(GetCurrentThreadId(), WM_QUIT,
                                       windows::Win32::Foundation::WPARAM(0),
                                       windows::Win32::Foundation::LPARAM(0));
        }

        Ok(())
    }

    fn get_monitors(&self) -> Result<Vec<MonitorInfo>> {
        Ok(vec![MonitorInfo {
            id: 0,
            name: "Primary Monitor".into(),
            width: 1920,
            height: 1080,
            refresh_rate: 60,
            is_primary: true,
        }])
    }
}