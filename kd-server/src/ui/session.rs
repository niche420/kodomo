use egui::Ui;
use crate::http::SharedState;
use crate::profile::load_profile;
use crate::state::SessionState;
use crate::ui::AppEvent;
use crate::ui::screen::{Screen, ScreenType};

pub struct SessionScreen {
    state: SharedState,
}

impl SessionScreen {
    pub fn new(state: SharedState) -> Self {
        Self { state }
    }
}

impl Screen for SessionScreen {
    fn on_show(&mut self) {
        let mut state = self.state.lock().unwrap();

        // Build SessionState from what the connect screen stored
        let game_title = match state.selected_game.clone() {
            Some(t) => t,
            None => {
                eprintln!("session: no game selected");
                return;
            }
        };
        let token = match state.session.as_ref().map(|s| s.token.clone()) {
            Some(t) => t,
            None => {
                eprintln!("session: no session token");
                return;
            }
        };
        let client_ip = match state.session.as_ref().map(|s| s.client_ip.clone()) {
            Some(ip) => ip,
            None => {
                eprintln!("session: no client ip");
                return;
            }
        };

        let session = SessionState {
            token,
            client_ip,
            game_title: game_title.clone(),
        };

        // Load the active profile for this game if one is set
        let profile = state.persistent.games
            .iter()
            .find(|g| g.metadata.title == game_title)
            .and_then(|g| g.active_profile.as_deref())
            .and_then(|name| load_profile(&game_title, name));

        state.push_event(AppEvent::PipelineStart(session, profile));
    }

    fn render(&mut self, ui: &mut Ui) {
        let mut state = self.state.lock().unwrap();

        ui.heading("Streaming");
        if let Some(session) = &state.session {
            ui.label(format!("Game: {}", session.game_title));
            ui.label(format!("Session: {}", &session.token[..8]));
        }

        ui.add_space(16.0);

        if ui.button("Stop Streaming").clicked() {
            state.push_event(AppEvent::PipelineEnd);
            state.push_event(AppEvent::ScreenTransition(ScreenType::Home));
        }
    }

    fn get_type(&self) -> ScreenType {
        ScreenType::Session
    }
}