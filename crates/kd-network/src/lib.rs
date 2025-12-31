use bytes::Bytes;
use std::net::SocketAddr;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;

pub mod protocol;
pub mod webrtc_transport;
pub mod udp_transport;
pub mod packet;

pub use protocol::{Message, MessageType};
pub use packet::{Packet, PacketType};

pub type Result<T> = std::result::Result<T, NetworkError>;

#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Receive failed: {0}")]
    ReceiveFailed(String),

    #[error("Transport not supported: {0}")]
    UnsupportedTransport(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("WebRTC error: {0}")]
    WebRTC(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportType {
    WebRTC,
    UDP,
}

#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub transport: TransportType,
    pub bind_addr: SocketAddr,
    pub max_packet_size: usize,
    pub buffer_size: usize,
    pub enable_fec: bool, // Forward Error Correction
    pub enable_retransmission: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            transport: TransportType::WebRTC,
            bind_addr: "0.0.0.0:8080".parse().unwrap(),
            max_packet_size: 1400, // MTU-safe
            buffer_size: 1024,
            enable_fec: false,
            enable_retransmission: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub packets_sent: u64,
    pub packets_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_lost: u64,
    pub rtt_ms: f64,
    pub jitter_ms: f64,
}

impl Default for NetworkStats {
    fn default() -> Self {
        Self {
            packets_sent: 0,
            packets_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            packets_lost: 0,
            rtt_ms: 0.0,
            jitter_ms: 0.0,
        }
    }
}

/// Trait for network transport implementations
#[async_trait::async_trait]
pub trait NetworkTransport: Send + Sync {
    async fn init(&mut self, config: NetworkConfig) -> Result<()>;
    async fn connect(&mut self, addr: SocketAddr) -> Result<()>;
    async fn send(&mut self, data: Bytes) -> Result<()>;
    async fn recv(&mut self) -> Result<Bytes>;
    async fn disconnect(&mut self) -> Result<()>;
    fn get_stats(&self) -> NetworkStats;
}

/// Factory to create appropriate transport
pub struct TransportFactory;

impl TransportFactory {
    pub fn create(transport_type: TransportType) -> Result<Box<dyn NetworkTransport>> {
        match transport_type {
            TransportType::WebRTC => {
                tracing::info!("Creating WebRTC transport");
                Ok(Box::new(webrtc_transport::WebRTCTransport::new()))
            }
            TransportType::UDP => {
                tracing::info!("Creating UDP transport");
                Ok(Box::new(udp_transport::UdpTransport::new()))
            }
        }
    }
}