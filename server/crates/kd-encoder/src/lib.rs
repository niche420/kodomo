use std::time::Instant;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod software;

#[cfg(feature = "ffmpeg")]
mod ffmpeg;

#[cfg(feature = "openh264")]
mod openh264_encoder;

#[cfg(feature = "nvenc")]
mod nvenc_encoder;

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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

        // Try FFmpeg first (supports both hardware NVENC and software encoding)
        #[cfg(feature = "ffmpeg")]
        {
            if config.use_hardware && ffmpeg::FfmpegEncoder::is_nvenc_available() {
                match ffmpeg::FfmpegEncoder::new(config.clone()) {
                    Ok(encoder) => {
                        tracing::info!("✓ Using FFmpeg with NVENC hardware acceleration");
                        return Ok(Box::new(encoder));
                    }
                    Err(e) => {
                        tracing::warn!("FFmpeg NVENC init failed: {}, trying software", e);
                    }
                }
            }

            // Try FFmpeg software encoding
            if ffmpeg::FfmpegEncoder::is_available() {
                match ffmpeg::FfmpegEncoder::new(config.clone()) {
                    Ok(encoder) => {
                        tracing::info!("✓ Using FFmpeg software encoder");
                        return Ok(Box::new(encoder));
                    }
                    Err(e) => {
                        tracing::warn!("FFmpeg init failed: {}", e);
                    }
                }
            }
        }

        // Try OpenH264 if available
        #[cfg(feature = "openh264")]
        {
            if openh264_encoder::OpenH264Encoder::is_available() {
                if let Ok(encoder) = openh264_encoder::OpenH264Encoder::new(config.clone()) {
                    tracing::info!("✓ Using OpenH264 software encoder");
                    return Ok(Box::new(encoder));
                }
            }
        }

        // Final fallback to stub encoder
        tracing::warn!("Using stub software encoder (visual output will be garbage)");
        Ok(Box::new(software::SoftwareEncoder::new(config)?))
    }

    pub fn list_available_encoders() -> Vec<String> {
        let mut encoders = vec![];

        #[cfg(feature = "ffmpeg")]
        {
            if ffmpeg::FfmpegEncoder::is_nvenc_available() {
                encoders.push("FFmpeg h264_nvenc (NVIDIA Hardware)".to_string());
            }
            if ffmpeg::FfmpegEncoder::is_available() {
                encoders.push("FFmpeg libx264 (Software)".to_string());
            }
        }

        #[cfg(feature = "openh264")]
        {
            if openh264_encoder::OpenH264Encoder::is_available() {
                encoders.push("OpenH264 (Software)".to_string());
            }
        }

        encoders.push("x264 Stub (Software Fallback)".to_string());

        encoders
    }
}