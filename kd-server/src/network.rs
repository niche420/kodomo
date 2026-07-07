pub mod worker;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct NetworkConfig {
    pub video_port: u16,
    pub handshake_port: u16,
    pub http_port: u16,
    pub input_port: u16
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            video_port: 5000,
            handshake_port: 6000,
            http_port: 7000,
            input_port: 8000,
        }
    }
}
