use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::capture;
use crate::session::common::FrameSlot;
use crate::session::SessionWorker;

pub struct CaptureWorker {
    game_exe_path: PathBuf,
    frame_slot: Arc<FrameSlot>,
    stopped: Arc<AtomicBool>,
}

impl CaptureWorker {
    pub fn new(game_exe_path: PathBuf, frame_slot: Arc<FrameSlot>, stopped: Arc<AtomicBool>) -> CaptureWorker {
        CaptureWorker {
            game_exe_path,
            frame_slot,
            stopped
        }
    }
}

impl SessionWorker for CaptureWorker {
    fn run(&mut self) {
        let mut capturer = match capture::create_capturer(&self.game_exe_path) {
            Ok(c) => c,
            Err(e) => { eprintln!("capture: {e}"); return; }
        };
        while !self.stopped.load(Ordering::SeqCst) {
            if let Some(frame) = capturer.capture_frame() {
                self.frame_slot.write(frame);
            }
        }
    }
}