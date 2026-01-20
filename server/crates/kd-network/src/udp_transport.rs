use super::*;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

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

        // No fragmentation - send as-is
        // If data is too large, it's the encoder's job to split NAL units
        let packet = Packet::new(PacketType::Video, self.sequence, data);
        self.sequence = self.sequence.wrapping_add(1);

        let wire_data = packet.to_bytes();

        // Single send - no loops, no fragmentation
        socket.send(&wire_data).await
            .map_err(|e| NetworkError::SendFailed(e.to_string()))?;

        // Update stats
        let mut stats = self.stats.lock().await;
        stats.packets_sent += 1;
        stats.bytes_sent += wire_data.len() as u64;

        Ok(())
    }

    async fn recv(&mut self) -> Result<Bytes> {
        let socket = self.socket.as_ref()
            .ok_or(NetworkError::ReceiveFailed("Socket not initialized".into()))?;

        let config = self.config.as_ref().unwrap();
        let mut buf = vec![0u8; config.max_packet_size + 128];

        let (len, addr) = socket.recv_from(&mut buf).await
            .map_err(|e| NetworkError::ReceiveFailed(e.to_string()))?;

        // Auto-connect to first client (server mode)
        if self.peer_addr.is_none() {
            info!("UDP server: first packet from {}, setting as peer", addr);
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
        stats.bytes_received += len as u64;

        // SIMPLIFIED: Return payload directly, no reassembly
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_udp_send_receive() {
        let mut server = UdpTransport::new();
        let mut client = UdpTransport::new();

        let server_config = NetworkConfig {
            bind_addr: "127.0.0.1:8080".parse().unwrap(),
            ..Default::default()
        };

        let client_config = NetworkConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            ..Default::default()
        };

        server.init(server_config).await.unwrap();
        client.init(client_config).await.unwrap();

        client.connect("127.0.0.1:8080".parse().unwrap()).await.unwrap();

        let test_data = Bytes::from(vec![1, 2, 3, 4, 5]);
        client.send(test_data.clone()).await.unwrap();

        let received = server.recv().await.unwrap();
        assert_eq!(received, test_data);
    }
}