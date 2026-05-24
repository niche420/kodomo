#[cfg(target_os = "windows")]
mod windows;

use ffmpeg_next::codec::traits::Encoder;
use serde::{Deserialize, Serialize};
use crate::capture::Frame;
use crate::encode::windows::FfmpegEncoder;

pub trait FrameEncoder
{
    fn encode_frame(&mut self, frame: Frame) -> anyhow::Result<Vec<Vec<u8>>>;
}

#[derive(Serialize, Deserialize, Default)]
pub struct EncodeConfig {
    fps: u32,
    width: u32,
    height: u32,
}

pub fn create_encoder(config: &EncodeConfig) -> anyhow::Result<Box<dyn FrameEncoder>> {
    #[cfg(target_os = "windows")]
    Ok(Box::new(FfmpegEncoder::new(config.width, config.height, config.fps)?))
}
