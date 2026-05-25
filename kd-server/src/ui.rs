mod home;
mod connect;
mod screen;
mod session;

use std::cell::RefCell;
use std::cmp::PartialEq;
use std::ops::Deref;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use eframe::App;
use serde::{Deserialize, Serialize};
use kd_shared::game::{GameMetadata};
use crate::pipeline::{Pipeline, PipelineConfig};
use crate::ui::connect::ConnectScreen;
use crate::ui::home::HomeScreen;
use crate::ui::screen::{Screen, ScreenType};
use crate::ui::session::SessionScreen;

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
    pub config: PipelineConfig,

    #[serde(skip)]
    pub selected_game: Option<String>,
    #[serde(skip)]
    session: Option<String>,
    #[serde(skip)]
    pipeline: Pipeline,
    #[serde(skip)]
    current_screen: ScreenType
}

impl AppState {
    pub fn transition_to(&mut self, screen: ScreenType) {
        self.current_screen = screen;
    }
    
    pub fn start_session(&mut self) -> anyhow::Result<()> {
        self.pipeline.start(self.config.clone())
    }
    
    pub fn end_session(&mut self) {
        self.pipeline.stop();
    }
}

pub struct ServerApp {
    state: Rc<RefCell<AppState>>,
    current_screen: Box<dyn Screen>,
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
                session: None,
                selected_game: None,
                pipeline: Pipeline::new(),
                current_screen: ScreenType::Home,
                config: PipelineConfig::default()
            }
        };
        let rc_state = Rc::new(RefCell::new(state));

        Self {
            state: rc_state.clone(),
            current_screen: Self::make_screen(rc_state, ScreenType::Home)
        }
    }

    fn make_screen(state: Rc<RefCell<AppState>>, screen: ScreenType) -> Box<dyn Screen> {
        match screen {
            ScreenType::Home => Box::new(HomeScreen::new(state)),
            ScreenType::Connect => Box::new(ConnectScreen::new(state)),
            ScreenType::Session => Box::new(SessionScreen::new(state)),
        }
    }
}

impl eframe::App for ServerApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let desired = self.state.borrow().current_screen.clone();
            if desired != self.current_screen.get_type() {
                self.current_screen = Self::make_screen(self.state.clone(), desired);
                self.current_screen.on_show();
            }
            self.current_screen.render(ui);
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self.state.borrow().deref());
    }
}