use std::any::Any;
use egui::Ui;
use crate::http::SharedState;
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
    fn get_type(&self) -> ScreenType { ScreenType::Session }
    fn as_any(&self) -> &dyn Any { self }

    fn render(&mut self, ui: &mut Ui) {
        let mut state = self.state.lock().unwrap();

        ui.heading("Streaming");
        if let Some(session) = &state.session {
            ui.label(format!("Game: {}", session.game_title));
            ui.label(format!("Clients: {}", session.clients.len()));
            for client in &session.clients {
                ui.label(format!("  {}", client.ip));
            }
        }

        ui.add_space(16.0);

        if ui.button("Stop Streaming").clicked() {
            state.push_event(AppEvent::PipelineEnd);
        }
    }
}