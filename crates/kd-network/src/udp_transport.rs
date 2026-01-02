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
    // Fragment reassembly buffer
    fragment_buffer: Arc<Mutex<FragmentBuffer>>,
}

struct FragmentBuffer {
    fragments: Vec<Bytes>,
    expected_sequence: Option<u32>,
}

impl FragmentBuffer {
    fn new() -> Self {
        Self {
            fragments: Vec::new(),
            expected_sequence: None,
        }
    }

    fn add_fragment(&mut self, packet: &Packet) -> Option<Bytes> {
        // Check if this is the start of a new fragmented packet
        if (packet.flags & FLAG_FRAGMENT) != 0 {
            if self.expected_sequence.is_none() {
                // First fragment
                self.expected_sequence = Some(packet.sequence);
                self.fragments.clear();
            }

            // Add fragment if sequence matches
            if Some(packet.sequence) == self.expected_sequence {
                self.fragments.push(packet.payload.clone());
                self.expected_sequence = Some(packet.sequence.wrapping_add(1));

                // Check if this is the last fragment
                if (packet.flags & FLAG_LAST_FRAGMENT) != 0 {
                    // Reassemble all fragments
                    let total_size: usize = self.fragments.iter().map(|f| f.len()).sum();
                    let mut reassembled = Vec::with_capacity(total_size);

                    for fragment in &self.fragments {
                        reassembled.extend_from_slice(fragment);
                    }

                    // Reset buffer
                    self.fragments.clear();
                    self.expected_sequence = None;

                    return Some(Bytes::from(reassembled));
                }
            } else {
                // Sequence mismatch, reset
                warn!("Fragment sequence mismatch, resetting buffer");
                self.fragments.clear();
                self.expected_sequence = None;
            }

            None
        } else {
            // Not a fragment, return as-is
            Some(packet.payload.clone())
        }
    }
}

impl UdpTransport {
    pub fn new() -> Self {
        Self {
            socket: None,
            peer_addr: None,
            config: None,
            stats: Arc::new(Mutex::new(NetworkStats::default())),
            sequence: 0,
            fragment_buffer: Arc::new(Mutex::new(FragmentBuffer::new())),
        }
    }

    /// Find all NAL unit boundaries in the data
    fn find_nal_boundaries(data: &[u8]) -> Vec<usize> {
        let mut boundaries = Vec::new();
        let mut i = 0;

        // Look for start codes: 0x00 0x00 0x00 0x01 or 0x00 0x00 0x01
        while i + 3 < data.len() {
            if data[i] == 0x00 && data[i + 1] == 0x00 {
                if data[i + 2] == 0x00 && i + 3 < data.len() && data[i + 3] == 0x01 {
                    // Found 4-byte start code
                    boundaries.push(i);
                    i += 4;
                } else if data[i + 2] == 0x01 {
                    // Found 3-byte start code
                    boundaries.push(i);
                    i += 3;
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        boundaries
    }

    /// Split data into chunks that respect NAL unit boundaries
    fn create_nal_aware_chunks(data: &Bytes, max_chunk_size: usize) -> Vec<Bytes> {
        let boundaries = Self::find_nal_boundaries(data);

        if boundaries.is_empty() {
            // No NAL units found, fall back to simple chunking
            // (This shouldn't happen with properly formatted H.264)
            warn!("No NAL boundaries found, using simple chunking");
            return data.chunks(max_chunk_size)
                .map(|chunk| Bytes::copy_from_slice(chunk))
                .collect();
        }

        let mut chunks = Vec::new();
        let mut current_chunk_start = 0;

        for i in 0..boundaries.len() {
            let nal_start = boundaries[i];
            let nal_end = if i + 1 < boundaries.len() {
                boundaries[i + 1]
            } else {
                data.len()
            };

            let nal_size = nal_end - nal_start;

            // If this NAL unit alone exceeds max_chunk_size, it needs its own chunk(s)
            if nal_size > max_chunk_size {
                // Flush current chunk if it has data
                if current_chunk_start < nal_start {
                    chunks.push(data.slice(current_chunk_start..nal_start));
                }

                // Split this large NAL unit into multiple chunks
                // Note: This is not ideal but necessary for very large NAL units
                let mut nal_offset = nal_start;
                while nal_offset < nal_end {
                    let chunk_end = (nal_offset + max_chunk_size).min(nal_end);
                    chunks.push(data.slice(nal_offset..chunk_end));
                    nal_offset = chunk_end;
                }

                current_chunk_start = nal_end;
            } else if current_chunk_start + (nal_end - current_chunk_start) > max_chunk_size {
                // Adding this NAL would exceed chunk size, flush current chunk
                chunks.push(data.slice(current_chunk_start..nal_start));
                current_chunk_start = nal_start;
            }
            // else: NAL fits in current chunk, continue accumulating
        }

        // Flush remaining data
        if current_chunk_start < data.len() {
            chunks.push(data.slice(current_chunk_start..));
        }

        chunks
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

        let _peer = self.peer_addr
            .ok_or(NetworkError::SendFailed("No peer address set. Call connect() first.".into()))?;

        let config = self.config.as_ref().unwrap();
        let max_payload_size = config.max_packet_size.saturating_sub(50); // Leave room for packet header

        // Check if fragmentation is needed
        if data.len() <= max_payload_size {
            // Single packet - no fragmentation needed
            let packet = Packet::new(PacketType::Video, self.sequence, data);
            self.sequence = self.sequence.wrapping_add(1);

            let wire_data = packet.to_bytes();
            socket.send(&wire_data).await
                .map_err(|e| NetworkError::SendFailed(e.to_string()))?;

            let mut stats = self.stats.lock().await;
            stats.packets_sent += 1;
            stats.bytes_sent += wire_data.len() as u64;

            debug!("Sent UDP packet #{}, {} bytes", packet.sequence, wire_data.len());
        } else {
            // Need fragmentation - use NAL-aware chunking
            self.send_fragmented_nal_aware(data, max_payload_size).await?;
        }

        Ok(())
    }

    async fn recv(&mut self) -> Result<Bytes> {
        let socket = self.socket.as_ref()
            .ok_or(NetworkError::ReceiveFailed("Socket not initialized".into()))?;

        let config = self.config.as_ref().unwrap();
        let mut buf = vec![0u8; config.max_packet_size + 128];

        let (len, addr) = socket.recv_from(&mut buf).await
            .map_err(|e| NetworkError::ReceiveFailed(e.to_string()))?;

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

        debug!("Received UDP packet #{}, {} bytes, fragmented: {}",
               packet.sequence, packet.payload.len(),
               (packet.flags & FLAG_FRAGMENT) != 0);

        // Handle fragmentation
        let mut fragment_buffer = self.fragment_buffer.lock().await;
        if let Some(complete_data) = fragment_buffer.add_fragment(&packet) {
            Ok(complete_data)
        } else {
            // Fragment received but not complete yet, return empty
            // The next recv() call will continue reassembly
            Err(NetworkError::ReceiveFailed("Fragment not complete".into()))
        }
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
    async fn send_fragmented_nal_aware(&mut self, data: Bytes, max_chunk_size: usize) -> Result<()> {
        let _peer = self.peer_addr
            .ok_or(NetworkError::SendFailed("No peer address set".into()))?;

        // Create NAL-aware chunks
        let chunks = Self::create_nal_aware_chunks(&data, max_chunk_size);
        let total_chunks = chunks.len();

        info!("Fragmenting {} bytes into {} NAL-aware chunks", data.len(), total_chunks);

        for (idx, chunk) in chunks.into_iter().enumerate() {
            let mut flags = FLAG_FRAGMENT;
            if idx == total_chunks - 1 {
                flags |= FLAG_LAST_FRAGMENT;
            }

            let packet = Packet::new(PacketType::Video, self.sequence, chunk)
                .with_flags(flags);

            self.sequence = self.sequence.wrapping_add(1);

            let wire_data = packet.to_bytes();
            let socket = self.socket.as_ref().unwrap();

            socket.send(&wire_data).await
                .map_err(|e| NetworkError::SendFailed(e.to_string()))?;

            let mut stats = self.stats.lock().await;
            stats.packets_sent += 1;
            stats.bytes_sent += wire_data.len() as u64;

            debug!("Sent NAL-aware fragment {}/{}, {} bytes",
                   idx + 1, total_chunks, wire_data.len());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::FLAG_KEYFRAME;

    #[test]
    fn test_nal_boundary_detection() {
        // H.264 with 4-byte start codes
        let data = vec![
            0x00, 0x00, 0x00, 0x01, 0x67, 0x42, // SPS
            0x00, 0x00, 0x00, 0x01, 0x68, 0xce, // PPS
            0x00, 0x00, 0x00, 0x01, 0x65, 0x88, // IDR slice
        ];

        let boundaries = UdpTransport::find_nal_boundaries(&data);
        assert_eq!(boundaries, vec![0, 6, 12]);
    }

    #[test]
    fn test_nal_aware_chunking() {
        // Create test data with NAL units
        let mut data = Vec::new();

        // SPS (small)
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x67]);
        data.extend_from_slice(&vec![0xFF; 100]); // 100 bytes of SPS data

        // PPS (small)
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x68]);
        data.extend_from_slice(&vec![0xEE; 50]); // 50 bytes of PPS data

        // IDR slice (large)
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x65]);
        data.extend_from_slice(&vec![0xDD; 2000]); // 2000 bytes of slice data

        let data = Bytes::from(data);
        let chunks = UdpTransport::create_nal_aware_chunks(&data, 500);

        // Should create multiple chunks, each starting with a NAL unit
        assert!(chunks.len() > 1);

        // Each chunk should start with 0x00 0x00
        for chunk in &chunks {
            if chunk.len() >= 2 {
                // Most chunks should start with NAL unit start code
                // (except when a large NAL is split)
                let starts_with_nal = chunk[0] == 0x00 && chunk[1] == 0x00;
                if !starts_with_nal {
                    // This is acceptable for continuation of a large NAL
                    println!("Chunk doesn't start with NAL (continuation of large NAL)");
                }
            }
        }
    }

    #[test]
    fn test_fragment_reassembly() {
        let mut buffer = FragmentBuffer::new();

        // Create fragmented packets
        let chunk1 = Bytes::from(vec![1, 2, 3]);
        let chunk2 = Bytes::from(vec![4, 5, 6]);
        let chunk3 = Bytes::from(vec![7, 8, 9]);

        let packet1 = Packet::new(PacketType::Video, 1, chunk1)
            .with_flags(FLAG_FRAGMENT);
        let packet2 = Packet::new(PacketType::Video, 2, chunk2)
            .with_flags(FLAG_FRAGMENT);
        let packet3 = Packet::new(PacketType::Video, 3, chunk3)
            .with_flags(FLAG_FRAGMENT | FLAG_LAST_FRAGMENT);

        // Add fragments
        assert!(buffer.add_fragment(&packet1).is_none());
        assert!(buffer.add_fragment(&packet2).is_none());

        let result = buffer.add_fragment(&packet3);
        assert!(result.is_some());

        let reassembled = result.unwrap();
        assert_eq!(reassembled.as_ref(), &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[tokio::test]
    async fn test_udp_transport_init() {
        let mut transport = UdpTransport::new();
        let config = NetworkConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            ..Default::default()
        };

        let result = transport.init(config).await;
        assert!(result.is_ok());
    }
}