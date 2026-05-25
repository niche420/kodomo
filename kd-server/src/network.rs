use std::io::{BufRead, BufReader, Write};
use std::net::SocketAddr;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct NetworkConfig {
    pub dest_ip: String,
    pub video_port: u16,
    pub handshake_port: u16,
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

    pub fn listen(&self) -> anyhow::Result<SocketAddr> {
        let listener = std::net::TcpListener::bind(
            format!("0.0.0.0:{}", self.port))?;
        let (mut stream, addr) = listener.accept()?;
        let mut reader = BufReader::new(&stream);
        let mut token = String::new();
        reader.read_line(&mut token)?;
        let token = token.trim();
        if token == self.expected_token {
            stream.write_all(b"ok\n")?;
        } else {
            stream.write_all(b"err\n")?;
        }

        Ok(addr)
    }
}