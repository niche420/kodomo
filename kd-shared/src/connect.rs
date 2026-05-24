use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ConnectParams
{
    ip: String,
    port: u16,
    session: String,
    game: String
}

impl ConnectParams {
    pub fn new(ip: String, port: u16, session: String, game: String) -> Self {
        Self { ip, port, session, game }
    }

    /// Encodes as a kodomo:// deep link for the client QR scanner
    pub fn to_url(&self) -> String {
        // Minimal percent-encoding for game title
        let game = self.game.replace('%', "%25").replace(' ', "%20").replace('&', "%26");
        format!("kodomo://{}:{}?session={}&game={}", self.ip, self.port, self.session, game)
    }
}