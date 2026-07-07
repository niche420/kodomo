use std::any::Any;
use std::path::PathBuf;
use crate::http::SharedState;
use crate::state::Game;
use crate::ui::AppEvent;
use crate::ui::screen::{Screen, ScreenType};
use kd_shared::game::GameMetadata;

pub struct GamesScreen {
    state: SharedState,
    selected_game_title: Option<String>,
}

impl GamesScreen {
    pub fn new(state: SharedState) -> Self {
        Self { state, selected_game_title: None }
    }
}

impl Screen for GamesScreen {
    fn render(&mut self, ui: &mut egui::Ui) {
        let mut state = self.state.lock().unwrap();
        let has_pending = !state.pending_clients.is_empty();

        ui.horizontal(|ui| {
            ui.heading("Games");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("+ Add").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Executable", &["exe"])
                        .pick_file()
                    {
                        let title = path.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        if !state.persistent.games.iter().any(|g| g.exe_path == path) {
                            state.persistent.games.push(Game {
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
        });

        ui.separator();

        let mut to_delete: Option<usize> = None;
        let mut stream_game: Option<Game> = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, game) in state.persistent.games.iter().enumerate() {
                let game_title = &game.metadata.title;
                let is_selected = self.selected_game_title.as_deref() == Some(&game_title);
                if ui.selectable_label(is_selected, game_title).clicked() {
                    self.selected_game_title = if is_selected { None } else { Some(game_title.clone()) };
                }

                if is_selected {
                    ui.indent(game_title, |ui| {
                        ui.horizontal(|ui| {
                            // Stream button — only enabled when clients are waiting
                            // and not already streaming
                            let stream_enabled = has_pending && !state.is_streaming();
                            ui.add_enabled_ui(stream_enabled, |ui| {
                                let label = if state.is_streaming() {
                                    "Streaming..."
                                } else if has_pending {
                                    "▶ Stream"
                                } else {
                                    "▶ Stream (no clients)"
                                };
                                if ui.button(label).clicked() {
                                    stream_game = Some(game.clone());
                                }
                            });

                            if ui.button("🗑 Delete").clicked() {
                                to_delete = Some(i);
                                self.selected_game_title = None;
                            }
                        });
                    });
                }
            }
        });

        drop(state);

        if let Some(i) = to_delete {
            self.state.lock().unwrap().persistent.games.remove(i);
        }

        if let Some(game) = stream_game {
            self.state.lock().unwrap().push_event( AppEvent::StartSession(game) );
        }
    }
    fn get_type(&self) -> ScreenType { ScreenType::Games }
}
