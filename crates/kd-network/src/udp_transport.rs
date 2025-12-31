use super::*;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};
use crate::packet::{FLAG_FRAGMENT, FLAG_LAST_FRAGMENT};

pub struct UdpTransport {
    socket: Option<Arc<UdpSocket>>,
    peer_addr: Option<SocketAddr>,
    config: Option<NetworkConfig>,
    stats: Arc<Mutex<NetworkStats>>,
    sequence: u32,
}

impl UdpTransport {
    pub fn new() -> Self {
        Self {
            socket: None,
            peer_addr: None,
            config: None,
            stats: Arc::new(Mutex::new(NetworkStats::default())),
            sequence: 0,
        }
    }
}

#[async_trait::async_trait]
impl NetworkTransport for UdpTransport {
    async fn init(&mut self, config: NetworkConfig) -> Result<()> {
        info!("Initializing UDP transport on {}", config.bind_addr);

        let socket = UdpSocket::bind(config.bind_addr).await?;

        // Set socket options for low latency
        socket.set_broadcast(false)?;

        self.socket = Some(Arc::new(socket));
        self.config = Some(config);

        info!("UDP socket bound successfully");
        Ok(())
    }

    async fn connect(&mut self, addr: SocketAddr) -> Result<()> {
        info!("Setting UDP peer to {}", addr);

        let socket = self.socket.as_ref()
            .ok_or(NetworkError::ConnectionFailed("Socket not initialized".into()))?;

        socket.connect(addr).await?;
        self.peer_addr = Some(addr);

        info!("UDP connected to {}", addr);
        Ok(())
    }

    async fn send(&mut self, data: Bytes) -> Result<()> {
        let socket = self.socket.as_ref()
            .ok_or(NetworkError::SendFailed("Socket not initialized".into()))?;

        // Ensure we have a peer address
        let _peer = self.peer_addr
            .ok_or(NetworkError::SendFailed("No peer address set. Call connect() first.".into()))?;

        let config = self.config.as_ref().unwrap();

        // Fragment large packets if needed
        if data.len() > config.max_packet_size {
            return self.send_fragmented(data).await;
        }

        // Create packet
        let packet = Packet::new(PacketType::Video, self.sequence, data);
        self.sequence = self.sequence.wrapping_add(1);

        let wire_data = packet.to_bytes();

        // Send
        socket.send(&wire_data).await
            .map_err(|e| NetworkError::SendFailed(e.to_string()))?;

        // Update stats
        let mut stats = self.stats.lock().await;
        stats.packets_sent += 1;
        stats.bytes_sent += wire_data.len() as u64;

        debug!("Sent UDP packet #{}, {} bytes", packet.sequence, wire_data.len());
        Ok(())
    }

    async fn recv(&mut self) -> Result<Bytes> {
        let socket = self.socket.as_ref()
            .ok_or(NetworkError::ReceiveFailed("Socket not initialized".into()))?;

        let config = self.config.as_ref().unwrap();
        let mut buf = vec![0u8; config.max_packet_size + 128]; // Extra space for headers

        let (len, addr) = socket.recv_from(&mut buf).await
            .map_err(|e| NetworkError::ReceiveFailed(e.to_string()))?;

        // If we don't have a peer address yet (server mode), set it from the first packet
        if self.peer_addr.is_none() {
            info!("UDP server: first packet from {}, setting as peer", addr);
            // Connect to this peer for future sends
            if let Err(e) = socket.connect(addr).await {
                warn!("Failed to connect to peer {}: {}", addr, e);
            } else {
                self.peer_addr = Some(addr);
                info!("UDP server: connected to client {}", addr);
            }
        }

        buf.truncate(len);

        // Parse packet
        let packet = Packet::from_bytes(Bytes::from(buf))
            .map_err(|e| NetworkError::ReceiveFailed(e))?;

        // Update stats
        let mut stats = self.stats.lock().await;
        stats.packets_received += 1;
        stats.bytes_received += packet.payload.len() as u64;

        debug!("Received UDP packet #{}, {} bytes", packet.sequence, packet.payload.len());

        Ok(packet.payload)
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Closing UDP connection");
        self.socket = None;
        self.peer_addr = None;
        Ok(())
    }

    fn get_stats(&self) -> NetworkStats {
        NetworkStats::default()
    }
}

impl UdpTransport {
    async fn send_fragmented(&mut self, data: Bytes) -> Result<()> {
        // Ensure we have a peer address
        let _peer = self.peer_addr
            .ok_or(NetworkError::SendFailed("No peer address set. Call connect() first.".into()))?;

        let config = self.config.as_ref().unwrap();
        let chunk_size = config.max_packet_size - 50; // Leave room for headers

        let chunks: Vec<_> = data.chunks(chunk_size)
            .enumerate()
            .collect();

        let total_chunks = chunks.len();

        for (idx, chunk) in chunks {
            let mut flags = FLAG_FRAGMENT;
            if idx == total_chunks - 1 {
                flags |= FLAG_LAST_FRAGMENT;
            }

            let packet = Packet::new(PacketType::Video, self.sequence, Bytes::from(chunk.to_vec()))
                .with_flags(flags);

            self.sequence = self.sequence.wrapping_add(1);

            let wire_data = packet.to_bytes();
            let socket = self.socket.as_ref().unwrap();

            socket.send(&wire_data).await
                .map_err(|e| NetworkError::SendFailed(e.to_string()))?;

            debug!("Sent fragment {}/{}", idx + 1, total_chunks);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::packet::FLAG_KEYFRAME;
    use super::*;

    #[test]
    fn test_packet_serialization() {
        let payload = Bytes::from(vec![1, 2, 3, 4, 5]);
        let packet = Packet::new(PacketType::Video, 42, payload.clone());

        let serialized = packet.to_bytes();
        let deserialized = Packet::from_bytes(serialized).unwrap();

        assert_eq!(deserialized.packet_type, PacketType::Video);
        assert_eq!(deserialized.sequence, 42);
        assert_eq!(deserialized.payload, payload);
    }

    #[test]
    fn test_packet_flags() {
        let payload = Bytes::from(vec![1, 2, 3]);
        let packet = Packet::new(PacketType::Video, 1, payload)
            .with_flags(FLAG_KEYFRAME);

        assert!(packet.is_keyframe());
    }

    #[tokio::test]
    async fn test_udp_transport_init() {
        let mut transport = UdpTransport::new();
        let config = NetworkConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(), // Random port
            ..Default::default()
        };

        let result = transport.init(config).await;
        assert!(result.is_ok());
    }
}