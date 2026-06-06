use std::path::PathBuf;
use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use kd_shared::game::GameMetadata;
use crate::encode::EncodeConfig;
use crate::network::NetworkConfig;
use crate::ui::AppEvent;

// ─── Persistent ───────────────────────────────────────────────────────────────
// Everything in here survives restarts. Serialized by eframe.

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
// Lives only while a stream is active. Never persisted.

#[derive(Debug, Clone)]
pub struct SessionState {
    pub token: String,
    pub client_ip: String,
    pub game_title: String,
}

// ─── AppState ─────────────────────────────────────────────────────────────────
// Runtime-only. Holds persistent state + transient UI/session state.

pub struct AppState {
    pub persistent: PersistentState,
    /// Which game is highlighted/selected in the UI. UI navigation only.
    pub selected_game: Option<String>,
    /// Set when a stream session is active, cleared when it ends.
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
            selected_game: None,
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