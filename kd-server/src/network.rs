use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, UdpSocket};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct NetworkConfig {
    pub video_port: u16,
    pub handshake_port: u16,
    pub http_port: u16,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            video_port: 5000,
            handshake_port: 6000,
            http_port: 7000,
        }
    }
}

pub struct HandshakeResult {
    pub client_ip: IpAddr,
    /// Already-bound UDP socket on the dynamically assigned input port.
    /// Hand this directly to the input thread — no re-bind needed.
    pub input_socket: UdpSocket,
}

pub struct HandshakeListener {
    port: u16,
    expected_token: String,
}

impl HandshakeListener {
    pub fn new(port: u16, expected_token: String) -> Self {
        Self { port, expected_token }
    }

    pub fn listen(&self) -> anyhow::Result<HandshakeResult> {
        let listener = std::net::TcpListener::bind(format!("0.0.0.0:{}", self.port))?;
        let (mut stream, addr) = listener.accept()?;
        let reader_stream = stream.try_clone()?;
        let mut reader = BufReader::new(reader_stream);

        let mut token = String::new();
        reader.read_line(&mut token)?;
        if token.trim() != self.expected_token {
            stream.write_all(b"err\n")?;
            return Err(anyhow::anyhow!("Token mismatch"));
        }

        // Bind before replying so port is held when client starts sending
        let input_socket = UdpSocket::bind("0.0.0.0:0")?;
        let input_port = input_socket.local_addr()?.port();

        stream.write_all(format!("ok:{}\n", input_port).as_bytes())?;

        let mut ready = String::new();
        reader.read_line(&mut ready)?;
        if ready.trim() != "ready" {
            return Err(anyhow::anyhow!("Expected ready"));
        }

        Ok(HandshakeResult { client_ip: addr.ip(), input_socket })
    }
}