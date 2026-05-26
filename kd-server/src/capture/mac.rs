use objc2::rc::Retained;
use objc2_screen_capture_kit::SCStream;
use crate::capture::{Frame, FrameCapturer};

pub struct ScreenCaptureKitCapturer {
    stream: Retained<SCStream>,
}

impl ScreenCaptureKitCapturer {
    pub fn new() -> anyhow::Result<ScreenCaptureKitCapturer> {
        Ok(Self {
            stream: unsafe { SCStream::new() }
        })
    }
}

impl FrameCapturer for ScreenCaptureKitCapturer {
    fn capture_frame(&mut self) -> Option<Frame> {
        todo!()
    }
}