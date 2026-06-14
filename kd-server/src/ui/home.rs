use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use kd_shared::game::GameMetadata;
use crate::http::SharedState;
use crate::state::Game;
use crate::ui::{AppEvent, AppState};
use crate::ui::screen::{Screen, ScreenType};

pub struct HomeScreen {
    state: SharedState,
}

impl HomeScreen {
    pub fn new(state: SharedState) -> HomeScreen {
        Self { state }
    }
}

impl Screen for HomeScreen {
    fn get_type(&self) -> ScreenType {
        ScreenType::Home
    }

    fn render(&mut self, ui: &mut egui::Ui) {
        let mut state = self.state.lock().unwrap();

        ui.horizontal(|ui| {
            ui.heading("Games");
            ui.add_space(8.0);
            if ui.button("+ Add Game").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Executable", &["exe"])
                    .pick_file()
                {
                    let title = path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    // Only add if not already registered
                    let mut persistent = &mut state.persistent;
                    if !persistent.games.iter().any(|g| g.exe_path == path) {
                        persistent.games.push(Game {
                            metadata: GameMetadata { title },
                            thumbnail: None,
                            exe_path: path,
                            is_running: false,
                            active_profile: None,
                        });
                    }
                }
            }
        });

        ui.separator();

        // Collect indices to delete outside the loop to avoid borrow issues
        let mut to_delete: Option<usize> = None;
        let mut stream_game: Option<String> = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, game) in state.persistent.games.iter().enumerate() {
                ui.horizontal(|ui| {
                    // Game title — click to stream
                    if ui.selectable_label(false, &game.metadata.title).clicked() {
                        stream_game = Some(game.metadata.title.clone());
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Trash — unregister game
                        if ui.button("🗑").on_hover_text("Unregister game").clicked() {
                            to_delete = Some(i);
                        }
                    });
                });
            }
        });

        // Handle deletion
        if let Some(i) = to_delete {
            state.persistent.games.remove(i);
        }

        // Handle stream navigation
        if let Some(title) = stream_game {
            state.selected_game = Some(title);
            state.push_event(AppEvent::ScreenTransition(ScreenType::Connect));
        }
    }
}