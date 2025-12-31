use std::time::Instant;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod software;
#[cfg(feature = "ffmpeg")]
pub mod ffmpeg_encoder;

#[cfg(all(target_os = "windows", feature = "nvenc"))]
pub mod nvenc;

#[cfg(all(target_os = "macos", feature = "videotoolbox"))]
pub mod videotoolbox;

pub type Result<T> = std::result::Result<T, EncoderError>;

#[derive(Debug, Error)]
pub enum EncoderError {
    #[error("Initialization failed: {0}")]
    InitFailed(String),

    #[error("Encoding failed: {0}")]
    EncodingFailed(String),

    #[error("Unsupported codec: {0:?}")]
    UnsupportedCodec(VideoCodec),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Hardware encoder not available")]
    HardwareUnavailable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VideoCodec {
    H264,
    H265,
    VP9,
}

impl std::fmt::Display for VideoCodec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VideoCodec::H264 => write!(f, "H.264"),
            VideoCodec::H265 => write!(f, "H.265"),
            VideoCodec::VP9 => write!(f, "VP9"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncoderPreset {
    UltraFast,
    SuperFast,
    VeryFast,
    Faster,
    Fast,
    Medium,
    Slow,
    Slower,
    VerySlow,
}

#[derive(Debug, Clone)]
pub struct EncoderConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub codec: VideoCodec,
    pub preset: EncoderPreset,
    pub keyframe_interval: u32,
    pub use_hardware: bool,
}

impl Default for EncoderConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 60,
            bitrate_kbps: 10000,
            codec: VideoCodec::H264,
            preset: EncoderPreset::Fast,
            keyframe_interval: 60,
            use_hardware: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RawFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: PixelFormat,
    pub pts: u64,
    pub timestamp: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    BGRA,
    RGBA,
    NV12,
    I420,
}

#[derive(Debug, Clone)]
pub struct EncodedPacket {
    pub data: Bytes,
    pub pts: u64,
    pub dts: u64,
    pub is_keyframe: bool,
    pub timestamp: Instant,
    pub codec: VideoCodec,
}

impl EncodedPacket {
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Trait for all video encoders
pub trait VideoEncoder: Send + Sync {
    fn init(&mut self, config: EncoderConfig) -> Result<()>;
    fn encode(&mut self, frame: &RawFrame) -> Result<Option<EncodedPacket>>;
    fn flush(&mut self) -> Result<Vec<EncodedPacket>>;
    fn set_bitrate(&mut self, bitrate_kbps: u32) -> Result<()>;
    fn get_config(&self) -> &EncoderConfig;
}

/// Factory to create the best available encoder
pub struct EncoderFactory;

impl EncoderFactory {
    pub fn create(config: EncoderConfig) -> Result<Box<dyn VideoEncoder>> {
        tracing::info!("Creating encoder: {:?}, hardware: {}", config.codec, config.use_hardware);

        // Try hardware encoders first if requested
        if config.use_hardware {
            #[cfg(all(target_os = "windows", feature = "nvenc"))]
            if let Ok(encoder) = nvenc::NvencEncoder::new(config.clone()) {
                tracing::info!("Using NVENC hardware encoder");
                return Ok(Box::new(encoder));
            }

            #[cfg(all(target_os = "macos", feature = "videotoolbox"))]
            if let Ok(encoder) = videotoolbox::VideoToolboxEncoder::new(config.clone()) {
                tracing::info!("Using VideoToolbox hardware encoder");
                return Ok(Box::new(encoder));
            }

            tracing::warn!("Hardware encoder not available, falling back to software");
        }

        // Fallback to software encoder
        tracing::info!("Using software encoder");
        Ok(Box::new(software::SoftwareEncoder::new(config)?))
    }

    pub fn list_available_encoders() -> Vec<String> {
        let mut encoders = vec!["Software (x264)".to_string()];

        #[cfg(all(target_os = "windows", feature = "nvenc"))]
        if nvenc::NvencEncoder::is_available() {
            encoders.push("NVENC (NVIDIA)".to_string());
        }

        #[cfg(all(target_os = "macos", feature = "videotoolbox"))]
        encoders.push("VideoToolbox (Apple)".to_string());

        encoders
    }
}