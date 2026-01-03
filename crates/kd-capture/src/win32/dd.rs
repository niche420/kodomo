use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use crate::{CaptureConfig, CaptureError, CaptureHandler, Result};

#[derive(Debug)]
pub struct DxgiDuplication {

}

impl DxgiDuplication {
    pub fn from_idx(idx: u32) -> Result<DxgiDuplication> {
        Ok(Self {
            
        })
    }
}

pub fn start_monitor_capture<H: CaptureHandler>(
    _config: CaptureConfig,
    _should_stop: &Arc<AtomicBool>,
    _handler: Arc<Mutex<H>>,
) -> Result<()> {
    // TODO: Implement DXGI Desktop Duplication
    Err(CaptureError::InitFailed("Desktop Duplication not yet implemented".into()))
}