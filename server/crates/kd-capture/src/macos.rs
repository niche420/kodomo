use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct CoreGraphicsCapture {
    should_stop: Arc<AtomicBool>,
}

impl CoreGraphicsCapture {
    pub fn new() -> Result<Self> {
        Ok(Self {
            should_stop: Arc::new(AtomicBool::new(false)),
        })
    }
}

impl ScreenCapture for CoreGraphicsCapture {
    fn start<H: CaptureHandler>(&mut self, config: CaptureConfig, mut handler: H) -> Result<()> {
        self.should_stop.store(false, Ordering::Relaxed);

        tracing::info!("Starting CoreGraphics capture: {:?}", config);

        macos::cg_capture::start_capture(
            config,
            &self.should_stop,
            &mut handler,
        )
    }

    fn stop(&self) -> Result<()> {
        tracing::info!("Stopping CoreGraphics capture");
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