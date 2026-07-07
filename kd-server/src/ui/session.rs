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

    fn render(&mut self, ui: &mut Ui) {
        let mut state = self.state.lock().unwrap();

        ui.heading("Streaming");

        if let Some(session) = state.session.as_ref() {
            ui.label(format!("Game: {}", session.game.metadata.title));
            ui.add_space(8.0);
            ui.label(format!("{} client(s):", session.num_clients()));
            for client in session.clients.lock().unwrap().iter() {
                ui.label(format!("  {}", client.ip));
            }

            ui.add_space(16.0);

            if ui.button("Stop").clicked() {
                state.push_event(AppEvent::EndSession);
            }
        } else {
            ui.label("No session is running. Scan the QR code on the connect page with the Kodomo app to start a session.");
        }
    }
}
