use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ConnectParams {
    ip: String,
    port: u16,
    session: String,
    handshake_port: u16,
    http_port: u16,
}

impl ConnectParams {
    pub fn new(
        ip: String,
        port: u16,
        session: String,
        handshake_port: u16,
        http_port: u16,
    ) -> Self {
        Self { ip, port, session, handshake_port, http_port }
    }

    pub fn to_url(&self) -> String {
        format!(
            "kodomo://{}:{}?session={}&handshake_port={}&http_port={}",
            self.ip, self.port, self.session, self.handshake_port, self.http_port
        )
    }
}