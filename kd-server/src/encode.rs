#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "macos")]
mod mac;

use serde::{Deserialize, Serialize};
use crate::capture::Frame;

pub trait FrameEncoder
{
    fn encode_frame(&mut self, frame: Frame) -> anyhow::Result<Vec<Vec<u8>>>;
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EncodeConfig {
    fps: u32,
    width: u32,
    height: u32,
}

impl Default for EncodeConfig {
    fn default() -> Self {
        Self {
            fps: 60,
            width: 1920,
            height: 1080,
        }
    }
}

pub fn create_encoder(config: &EncodeConfig) -> anyhow::Result<Box<dyn FrameEncoder>> {
    #[cfg(target_os = "windows")]
    return Ok(Box::new(windows::FfmpegEncoder::new(config.width, config.height, config.fps)?));

    #[cfg(target_os = "macos")]
    Ok(Box::new(mac::VideoToolboxEncoder::new(config.width, config.height, config.fps)?))
}
