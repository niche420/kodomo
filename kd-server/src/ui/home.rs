use std::cell::{RefCell, RefMut};
use std::ffi::OsStr;
use std::path::PathBuf;
use std::rc::Rc;
use egui::TextureOptions;
use uuid::Uuid;
use kd_shared::game::GameMetadata;
use crate::ui::{AppState, Game};
use crate::ui::screen::{Screen, ScreenType};

pub struct HomeScreen {
    state: Rc<RefCell<AppState>>
}

impl HomeScreen {
    pub fn new(state: Rc<RefCell<AppState>>) -> HomeScreen {
        Self {
            state
        }
    }
}

impl Screen for HomeScreen {
    fn get_type(&self) -> ScreenType {
        ScreenType::Home
    }

    fn render(&mut self, ui: &mut egui::Ui) {
        if ui.button("Add Game").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Executable", &["exe"])
                .pick_file() {
                self.state.borrow_mut().games.push(Game {
                    metadata: GameMetadata {
                        title: path.file_name().unwrap().to_str().unwrap().to_string()
                    },
                    thumbnail: None,
                    exe_path: path.clone(),
                    is_running: false,
                })
            }
        }
        ui.separator();

        let games = {
            let state = self.state.borrow();
            state.games.to_vec()
        };
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Active Games");
            {
                let state = self.state.borrow_mut();
                let active_games = games.iter().filter(|g| g.is_running).collect();
                show_games(state, active_games, ui);
            }

            ui.separator();

            ui.heading("Available Games");
            {
                let state = self.state.borrow_mut();
                let available_games = games.iter().filter(|g| !g.is_running).collect();
                show_games(state, available_games, ui);
            }
        });
    }
}

fn show_games(mut state: RefMut<AppState>, games: Vec<&Game>, ui: &mut egui::Ui) {
    for game in games {
        ui.horizontal(|ui| {
            if let Some(thumbnail) = &game.thumbnail {
                todo!();
                //ui.load_texture(thumbnail, _, TextureOptions::NEAREST);
                //ui.image(thumbnail);
            }
            if ui.selectable_label(false, &game.metadata.title).clicked() {
                state.selected_game = Some(game.metadata.title.clone());
                state.transition_to(ScreenType::Connect);
            }
        });
    }
}