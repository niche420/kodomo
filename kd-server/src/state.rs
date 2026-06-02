use std::path::PathBuf;
use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use kd_shared::game::GameMetadata;
use crate::pipeline::PipelineConfig;
use crate::ui::AppEvent;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Game {
    pub(crate) metadata: GameMetadata,
    pub(crate) thumbnail: Option<PathBuf>,
    pub(crate) exe_path: PathBuf,
    pub(crate) is_running: bool,
    pub(crate) active_profile: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[derive(Default)]
pub struct PersistentState {
    pub games: Vec<Game>,
    pub(crate) config: PipelineConfig,
}

pub struct AppState {
    pub(crate) persistent: PersistentState,
    pub selected_game: Option<String>,
    pub(crate) session: Option<String>,
    event_sender: Sender<AppEvent>,
    pub(crate) ctx: egui::Context,
}

impl AppState {
    pub fn new(persistent: PersistentState, event_sender: Sender<AppEvent>, ctx: egui::Context) -> Self {
        AppState {
            persistent,
            session: None,
            selected_game: None,
            event_sender,
            ctx
        }
    }

    pub fn push_event(&mut self, event: AppEvent) {
        // We need a repaint to get the new screen to show up
        if matches!(event, AppEvent::ScreenTransition(_)) {
            self.ctx.request_repaint();
        }
        self.event_sender.send(event).unwrap();
    }

    /// Find a game by title.
    pub fn game(&self, title: &str) -> Option<&Game> {
        self.persistent.games
            .iter()
            .find(|g| g.metadata.title == title)
    }

    pub fn game_mut(&mut self, title: &str) -> Option<&mut Game> {
        self.persistent.games
            .iter_mut()
            .find(|g| g.metadata.title == title)
    }
}