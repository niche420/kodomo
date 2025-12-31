#[cfg(target_os = "macos")]
pub mod macos {
    use super::*;
    use core_graphics::display::*;
    use tracing::info;

    pub struct CoreGraphicsCapture {
        display_id: CGDirectDisplayID,
        config: Option<CaptureConfig>,
    }

    impl CoreGraphicsCapture {
        pub fn new() -> Result<Self> {
            Ok(Self {
                display_id: CGMainDisplayID(),
                config: None,
            })
        }
    }

    impl ScreenCapture for CoreGraphicsCapture {
        fn init(&mut self, config: CaptureConfig) -> Result<()> {
            info!("Initializing macOS CoreGraphics capture: {:?}", config);
            self.config = Some(config);
            Ok(())
        }

        fn capture_frame(&mut self) -> Result<CapturedFrame> {
            let image = CGDisplay::screenshot(
                CGRectNull,
                kCGWindowListOptionOnScreenOnly,
                kCGNullWindowID,
                kCGWindowImageDefault,
            ).ok_or(CaptureError::CaptureFailed("CGDisplayCreateImage failed".into()))?;

            let width = image.width() as u32;
            let height = image.height() as u32;
            let bytes_per_row = image.bytes_per_row() as u32;

            // Copy pixel data
            let data_provider = image.data_provider();
            let data = data_provider.data();
            let data_vec = data.bytes().to_vec();

            Ok(CapturedFrame {
                data: data_vec,
                width,
                height,
                stride: bytes_per_row,
                format: PixelFormat::BGRA,
                timestamp: Instant::now(),
            })
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

        fn shutdown(&mut self) -> Result<()> {
            info!("Shutting down macOS capture");
            Ok(())
        }
    }
}