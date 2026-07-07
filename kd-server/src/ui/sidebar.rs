use egui::{Context, Panel, ScrollArea, Ui};
use crate::http::SharedState;
use crate::state::AppState;
use crate::ui::AppEvent;
use crate::ui::screen::{Screen, ScreenType};

pub struct Sidebar {
    state: SharedState,
}

impl Sidebar {
    pub fn new(state: SharedState) -> Self {
        Self {
            state
        }
    }

    pub fn render(&mut self, ui: &mut Ui) {
        Panel::left("sidebar")
            .resizable(true)
            .show_inside(ui, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    let mut state = self.state.lock().unwrap();
                    if ui.button("Games").clicked() {
                        state.push_event(AppEvent::ScreenTransition(ScreenType::Games));
                    }
                    if ui.button("Connect").clicked() {
                        state.push_event(AppEvent::ScreenTransition(ScreenType::Connect));
                    }
                    if ui.button("Network").clicked() {
                        state.push_event(AppEvent::ScreenTransition(ScreenType::Session));
                    }
                });
            });
    }
}
