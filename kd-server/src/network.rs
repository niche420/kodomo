use std::io::{BufRead, BufReader, Write};
use std::net::IpAddr;
use serde::{Deserialize, Serialize};

/// User-configurable network ports. Persisted.
/// dest_ip is NOT here — it is discovered at runtime during the handshake.
#[derive(Serialize, Deserialize, Clone)]
pub struct NetworkConfig {
    pub video_port: u16,
    pub handshake_port: u16,
    pub http_port: u16,
    pub input_port: u16,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            video_port: 5000,
            handshake_port: 6000,
            http_port: 7000,
            input_port: 5001,
        }
    }
}

pub struct HandshakeListener {
    port: u16,
    expected_token: String,
}

impl HandshakeListener {
    pub fn new(port: u16, expected_token: String) -> Self {
        Self { port, expected_token }
    }

    /// Accepts one connection, validates the token, returns the client IP.
    pub fn listen(&self) -> anyhow::Result<IpAddr> {
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

        stream.write_all(b"ok\n")?;

        let mut ready = String::new();
        reader.read_line(&mut ready)?;
        if ready.trim() != "ready" {
            return Err(anyhow::anyhow!("Expected ready"));
        }

        Ok(addr.ip())
    }
}