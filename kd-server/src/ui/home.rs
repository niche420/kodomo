use kd_shared::game::GameMetadata;
use crate::http::SharedState;
use crate::state::Game;
use crate::ui::AppEvent;
use crate::ui::screen::{Screen, ScreenType};
use std::any::Any;

pub struct HomeScreen {
    state: SharedState,
}

impl HomeScreen {
    pub fn new(state: SharedState) -> Self {
        Self { state }
    }
}

impl Screen for HomeScreen {
    fn get_type(&self) -> ScreenType { ScreenType::Home }
    fn as_any(&self) -> &dyn Any { self }

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

        ui.separator();

        let mut to_delete: Option<usize> = None;
        let mut navigate: Option<(String, std::path::PathBuf)> = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, game) in state.persistent.games.iter().enumerate() {
                ui.horizontal(|ui| {
                    if ui.selectable_label(false, &game.metadata.title).clicked() {
                        navigate = Some((game.metadata.title.clone(), game.exe_path.clone()));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🗑").on_hover_text("Unregister game").clicked() {
                            to_delete = Some(i);
                        }
                    });
                });
            }
        });

        if let Some(i) = to_delete {
            state.persistent.games.remove(i);
        }

        if let Some((game_title, exe_path)) = navigate {
            state.push_event(AppEvent::NavigateToConnect {
                game_title,
                exe_path,
                token: None,
            });
        }
    }
}