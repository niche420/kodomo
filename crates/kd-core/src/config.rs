use serde::{Deserialize, Serialize};
use kd_encoder::VideoCodec;
use kd_network::TransportType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    pub video: VideoConfig,
    pub audio: AudioConfig,
    pub network: NetworkConfig,
    pub input: InputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub codec: VideoCodec,
    pub hw_accel: bool,
    pub keyframe_interval: u32, // Keyframe every N frames
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub enabled: bool,
    pub sample_rate: u32,
    pub channels: u32,
    pub bitrate_kbps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub transport: TransportType,
    pub port: u16,
    pub max_packet_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    pub mouse_enabled: bool,
    pub keyboard_enabled: bool,
    pub gamepad_enabled: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            video: VideoConfig {
                width: 1920,
                height: 1080,
                fps: 60,
                bitrate_kbps: 10000,
                codec: VideoCodec::H264,
                hw_accel: true,
                keyframe_interval: 60,
            },
            audio: AudioConfig {
                enabled: true,
                sample_rate: 48000,
                channels: 2,
                bitrate_kbps: 128,
            },
            network: NetworkConfig {
                transport: TransportType::WebRTC,
                port: 8080,
                max_packet_size: 1400,
            },
            input: InputConfig {
                mouse_enabled: true,
                keyboard_enabled: true,
                gamepad_enabled: true,
            },
        }
    }
}