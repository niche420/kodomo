use bytes::Bytes;
use std::time::Instant;
use kd_encoder::VideoCodec;

#[derive(Debug, Clone)]
pub struct Frame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: PixelFormat,
    pub timestamp: Instant,
    pub frame_number: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    BGRA,
    RGBA,
    NV12,
    I420,
}

impl Frame {
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

#[derive(Debug, Clone)]
pub struct EncodedPacket {
    pub data: Bytes,
    pub timestamp: Instant,
    pub frame_number: u64,
    pub is_keyframe: bool,
    pub codec: VideoCodec,
}

impl EncodedPacket {
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub data: Vec<u8>,
    pub sample_rate: u32,
    pub channels: u32,
    pub timestamp: Instant,
}