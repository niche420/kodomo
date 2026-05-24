use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct NetworkConfig {
    pub dest_ip: String,
    pub video_port: u16,
    pub handshake_port: u16,
}