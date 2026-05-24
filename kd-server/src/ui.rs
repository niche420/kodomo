mod home;
mod connect;
mod screen;

use std::cell::RefCell;
use std::ops::Deref;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use kd_shared::game::{GameMetadata};
use crate::pipeline::{Pipeline, PipelineConfig};
use crate::ui::connect::ConnectScreen;
use crate::ui::home::HomeScreen;
use crate::ui::screen::{Screen, ScreenType};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Game {
    metadata: GameMetadata,
    thumbnail: Option<PathBuf>,
    exe_path: PathBuf,
    is_running: bool
}

#[derive(Serialize, Deserialize, Default)]
pub struct AppState {
    pub games: Vec<Game>,
    #[serde(skip)]
    pub screen: ScreenType,
    #[serde(skip)]
    pub selected_game: Option<String>,
    pub config: PipelineConfig,
    #[serde(skip)]
    pub pipeline: Arc<Pipeline>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ServerApp {
    state: Rc<RefCell<AppState>>,
    #[serde(skip)]
    screens: Vec<Box<dyn Screen>>
}

impl ServerApp {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let state = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            let mut games = Vec::new();
            games.push(Game {
                metadata: GameMetadata {
                    title: "Yakuza 3".to_string()
                },
                thumbnail: None,
                exe_path: PathBuf::new(),
                is_running: true
            });

            AppState {
                games,
                screen: ScreenType::Home,
                pipeline: Arc::new(Pipeline::new()),
                ..Default::default()
            }
        };
        let rc_state = Rc::new(RefCell::new(state));

        Self {
            state: rc_state.clone(),
            screens: vec![
                Box::new(HomeScreen::new(rc_state.clone())),
                Box::new(ConnectScreen::new(rc_state))
            ]
        }
    }
}

impl eframe::App for ServerApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let screen_idx = self.state.borrow().screen.clone() as usize;
            self.screens[screen_idx].render(ui);
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self.state.borrow().deref());
    }
}