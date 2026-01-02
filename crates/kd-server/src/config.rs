use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use kd_capture::{CaptureConfig, CaptureMode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub video: VideoConfig,
    pub audio: AudioConfig,
    pub network: NetworkConfig,
    pub capture: CaptureConfig,
    pub input: InputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub name: String,
    pub max_clients: u32,
    pub metrics_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub codec: Codec,
    pub preset: Preset,
    pub hw_accel: bool,
    pub keyframe_interval: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Codec {
    H264,
    H265,
    VP9,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Preset {
    UltraFast,
    SuperFast,
    VeryFast,
    Faster,
    Fast,
    Medium,
    Slow,
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
    pub transport: Transport,
    pub port: u16,
    pub bind_address: String,
    pub max_packet_size: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    WebRTC,
    UDP,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    pub keyboard_enabled: bool,
    pub mouse_enabled: bool,
    pub gamepad_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                name: "Game Streaming Server".into(),
                max_clients: 4,
                metrics_port: Some(9090),
            },
            video: VideoConfig {
                width: 1920,
                height: 1080,
                fps: 60,
                bitrate_kbps: 10000,
                codec: Codec::H264,
                preset: Preset::Fast,
                hw_accel: true,
                keyframe_interval: 60,
            },
            audio: AudioConfig {
                enabled: false, // TODO: Implement audio
                sample_rate: 48000,
                channels: 2,
                bitrate_kbps: 128,
            },
            network: NetworkConfig {
                transport: Transport::UDP,
                port: 8080,
                bind_address: "0.0.0.0".into(),
                max_packet_size: 1400,
            },
            capture: CaptureConfig {
                mode: CaptureMode::Unknown,
                width: 0,
                height: 0,
                fps: 0,
            },
            input: InputConfig {
                keyboard_enabled: true,
                mouse_enabled: true,
                gamepad_enabled: true,
            },
        }
    }
}

impl Config {
    pub fn from_file(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .context("Failed to read config file")?;

        let config: Config = toml::from_str(&contents)
            .context("Failed to parse config file")?;

        Ok(config)
    }

    pub fn to_file(&self, path: &Path) -> Result<()> {
        let contents = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        fs::write(path, contents)
            .context("Failed to write config file")?;

        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        if self.video.width == 0 || self.video.height == 0 {
            anyhow::bail!("Invalid video resolution");
        }

        if self.video.fps == 0 || self.video.fps > 240 {
            anyhow::bail!("Invalid FPS (must be 1-240)");
        }

        if self.video.bitrate_kbps < 1000 || self.video.bitrate_kbps > 100000 {
            anyhow::bail!("Invalid bitrate (must be 1000-100000 kbps)");
        }

        if self.network.port == 0 {
            anyhow::bail!("Invalid port");
        }

        Ok(())
    }
}