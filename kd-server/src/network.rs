use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct NetworkConfig {
    pub dest_ip: String,
    pub video_port: u16
}