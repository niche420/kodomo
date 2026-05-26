use std::cell::RefCell;
use std::rc::Rc;
use egui::Ui;
use crate::ui::{AppEvent, AppState};
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
        self.state.borrow_mut().push_event(AppEvent::PipelineStart);
    }

    fn render(&mut self, ui: &mut Ui) {
        let state = self.state.borrow();

        ui.heading("Streaming");
        ui.label(format!("Game: {}", state.selected_game.clone().unwrap_or_default()));
        ui.label(format!("Session: {}", &state.session.clone().unwrap_or_default()[..8]));

        ui.add_space(16.0);

        drop(state); // release borrow before mutating
        if ui.button("Stop Streaming").clicked() {
            self.state.borrow_mut().push_event(AppEvent::PipelineEnd);
            self.state.borrow_mut().push_event(AppEvent::ScreenTransition(ScreenType::Home));
        }
    }

    fn get_type(&self) -> ScreenType {
        ScreenType::Session
    }
}