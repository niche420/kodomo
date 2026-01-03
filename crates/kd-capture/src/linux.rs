use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct X11Capture {
    should_stop: Arc<AtomicBool>,
}

impl X11Capture {
    pub fn new() -> Result<Self> {
        Ok(Self {
            should_stop: Arc::new(AtomicBool::new(false)),
        })
    }
}

impl ScreenCapture for X11Capture {
    fn start<H: CaptureHandler>(&mut self, config: CaptureConfig, mut handler: H) -> Result<()> {
        self.should_stop.store(false, Ordering::Relaxed);

        tracing::info!("Starting X11 capture: {:?}", config);

        // Start polling-based capture
        linux::x11_capture::start_capture(
            config,
            &self.should_stop,
            &mut handler,
        )
    }

    fn stop(&self) -> Result<()> {
        tracing::info!("Stopping X11 capture");
        self.should_stop.store(true, Ordering::Relaxed);
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