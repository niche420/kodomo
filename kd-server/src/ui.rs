mod home;
mod connect;
mod screen;
mod session;

use std::cell::RefCell;
use std::cmp::PartialEq;
use std::collections::VecDeque;
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

pub enum AppEvent {
    ScreenTransition(ScreenType),
    PipelineStart,
    PipelineEnd,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Game {
    metadata: GameMetadata,
    thumbnail: Option<PathBuf>,
    exe_path: PathBuf,
    is_running: bool
}

pub struct AppState {
    persistent: PersistentState,
    pub selected_game: Option<String>,
    session: Option<String>,
    event_sender: crossbeam_channel::Sender<AppEvent>,
    ctx: egui::Context,
}

#[derive(Serialize, Deserialize)]
#[derive(Default)]
pub struct PersistentState {
    games: Vec<Game>,
    config: PipelineConfig,
}

impl AppState {
    pub fn push_event(&mut self, event: AppEvent) {
        // We need a repaint to get the new screen to show up
        if matches!(event, AppEvent::ScreenTransition(_)) {
            self.ctx.request_repaint();
        }
        self.event_sender.send(event).unwrap();
    }
}

pub struct ServerApp {
    state: Rc<RefCell<AppState>>,
    current_screen: Box<dyn Screen>,
    pipeline: Pipeline,
    event_receiver: crossbeam_channel::Receiver<AppEvent>
}

impl ServerApp {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (send, recv) = crossbeam_channel::unbounded();
        let persistent = if let Some(storage) = cc.storage {
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

            PersistentState {
                games,
                config: PipelineConfig::default()
            }
        };
        let rc_state = Rc::new(RefCell::new(
            AppState {
                persistent,
                session: None,
                selected_game: None,
                event_sender: send,
                ctx: cc.egui_ctx.clone()
            }
        ));

        Self {
            state: rc_state.clone(),
            current_screen: Self::make_screen(rc_state, ScreenType::Home),
            pipeline: Pipeline::new(),
            event_receiver: recv
        }
    }

    fn make_screen(state: Rc<RefCell<AppState>>, screen: ScreenType) -> Box<dyn Screen> {
        match screen {
            ScreenType::Home => Box::new(HomeScreen::new(state)),
            ScreenType::Connect => Box::new(ConnectScreen::new(state)),
            ScreenType::Session => Box::new(SessionScreen::new(state)),
        }
    }

    fn process_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.event_receiver.try_recv() {
            match event {
                AppEvent::ScreenTransition(after) => {
                    if after != self.current_screen.get_type() {
                        self.current_screen = Self::make_screen(self.state.clone(), after);
                        self.current_screen.on_show();
                        ctx.request_repaint();
                    }
                },
                AppEvent::PipelineStart => {
                    let state = self.state.borrow();
                    self.pipeline.start(state.persistent.config.clone());
                },
                AppEvent::PipelineEnd => {
                    self.pipeline.stop();
                }
            }
        }
    }
}

impl eframe::App for ServerApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.current_screen.render(ui);
        });

        self.process_events(ui.ctx());
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.state.borrow().persistent);
    }
}