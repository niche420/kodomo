use std::cell::RefCell;
use std::rc::Rc;
use egui::Ui;
use crate::ui::AppState;
use crate::ui::screen::{Screen, ScreenType};

pub struct SessionScreen {
    state: Rc<RefCell<AppState>>,
}

impl SessionScreen {
    pub fn new(state: Rc<RefCell<AppState>>) -> Self {
        Self {
            state
        }
    }
}

impl Screen for SessionScreen {
    fn on_show(&mut self) {
        if let Err(e) = self.state.borrow_mut().start_session() {
            eprintln!("Pipeline error: {e}");
        }
    }

    fn render(&mut self, ui: &mut Ui) {
        let state = self.state.borrow();

        ui.heading("Streaming");
        ui.label(format!("Game: {}", state.selected_game.clone().unwrap_or_default()));
        ui.label(format!("Session: {}", &state.session.clone().unwrap_or_default()[..8]));

        ui.add_space(16.0);

        drop(state); // release borrow before mutating
        if ui.button("Stop Streaming").clicked() {
            self.state.borrow_mut().end_session();
            self.state.borrow_mut().transition_to(ScreenType::Home);
        }
    }

    fn get_type(&self) -> ScreenType {
        ScreenType::Session
    }
}