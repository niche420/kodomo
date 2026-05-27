use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, SocketAddr};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct NetworkConfig {
    pub dest_ip: String,
    pub video_port: u16,
    pub handshake_port: u16,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            dest_ip: String::new(),
            video_port: 5000,
            handshake_port: 6000,
        }
    }
}

pub struct HandshakeListener {
    port: u16,
    expected_token: String,
}

impl HandshakeListener {
    pub fn new(port: u16, expected_token: String) -> Self {
        Self {
            port,
            expected_token
        }
    }

    pub fn listen(&self) -> anyhow::Result<IpAddr> {
        let listener = std::net::TcpListener::bind(
            format!("0.0.0.0:{}", self.port))?;
        let (mut stream, addr) = listener.accept()?;
        let mut reader = BufReader::new(&stream);
        let mut token = String::new();
        reader.read_line(&mut token)?;
        let token = token.trim();
        if token == self.expected_token {
            stream.write_all(b"ok\n")?;
            let mut ready = String::new();
            reader.read_line(&mut ready)?;
            if ready.trim() == "ready" {
                return Ok(addr.ip());
            }
        } else {
            stream.write_all(b"err\n")?;
        }

        Err(anyhow::Error::msg("Failed handshake"))
    }
}