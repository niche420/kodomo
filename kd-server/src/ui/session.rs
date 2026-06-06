use egui::Ui;
use crate::http::SharedState;
use crate::ui::AppEvent;
use crate::ui::screen::{Screen, ScreenType};

pub struct SessionScreen {
    state: SharedState,
}

impl SessionScreen {
    pub fn new(state: SharedState) -> Self {
        Self {
            state
        }
    }
}

impl Screen for SessionScreen {
    fn on_show(&mut self) {
        let mut state = self.state.lock().unwrap();
        state.persistent.config.active_game = state.selected_game;
        state.persistent.config.active_profile_name = state.
        state.push_event(AppEvent::PipelineStart);
    }

    fn render(&mut self, ui: &mut Ui) {
        let mut state = self.state.lock().unwrap();

        ui.heading("Streaming");
        ui.label(format!("Game: {}", state.selected_game.clone().unwrap_or_default()));
        ui.label(format!("Session: {}", &state.session.clone().unwrap_or_default()[..8]));

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