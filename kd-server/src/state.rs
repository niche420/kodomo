use std::net::UdpSocket;
use std::path::PathBuf;
use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use kd_shared::game::GameMetadata;
use kd_shared::profile::GameProfile;
use crate::encode::EncodeConfig;
use crate::network::NetworkConfig;
use crate::ui::AppEvent;

// ─── Persistent ───────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Default)]
pub struct PersistentState {
    pub games: Vec<Game>,
    pub network: NetworkConfig,
    pub encode: EncodeConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Game {
    pub metadata: GameMetadata,
    pub exe_path: PathBuf,
    pub thumbnail: Option<PathBuf>,
    pub active_profile: Option<String>,
    pub is_running: bool,
}

// ─── Session ──────────────────────────────────────────────────────────────────

pub struct ClientSession {
    pub ip: String,
    pub profile: Option<GameProfile>,
    /// Already-bound UDP socket for this client's input stream.
    pub input_socket: UdpSocket,
}

pub struct SessionState {
    pub game_title: String,
    pub exe_path: PathBuf,
    pub clients: Vec<ClientSession>,
}

// ─── AppState ─────────────────────────────────────────────────────────────────

pub struct AppState {
    pub persistent: PersistentState,
    pub session: Option<SessionState>,
    pub(crate) ctx: egui::Context,
    event_sender: Sender<AppEvent>,
}

impl AppState {
    pub fn new(
        persistent: PersistentState,
        event_sender: Sender<AppEvent>,
        ctx: egui::Context,
    ) -> Self {
        Self {
            persistent,
            session: None,
            ctx,
            event_sender,
        }
    }

    pub fn push_event(&mut self, event: AppEvent) {
        if matches!(event, AppEvent::ScreenTransition(_)) {
            self.ctx.request_repaint();
        }
        self.event_sender.send(event).unwrap();
    }

    pub fn game(&self, title: &str) -> Option<&Game> {
        self.persistent.games.iter().find(|g| g.metadata.title == title)
    }

    pub fn game_mut(&mut self, title: &str) -> Option<&mut Game> {
        self.persistent.games.iter_mut().find(|g| g.metadata.title == title)
    }
}