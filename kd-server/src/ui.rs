mod home;
mod connect;
mod screen;
mod session;

use std::cmp::PartialEq;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use eframe::App;
use serde::{Deserialize, Serialize};
use crate::pipeline::{Pipeline};
use crate::state::{AppState, PersistentState};
use crate::ui::connect::ConnectScreen;
use crate::ui::home::HomeScreen;
use crate::ui::screen::{Screen, ScreenType};
use crate::ui::session::SessionScreen;

pub enum AppEvent {
    ScreenTransition(ScreenType),
    PipelineStart,
    PipelineEnd,
}

pub struct ServerApp {
    state: Arc<Mutex<AppState>>,
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
            PersistentState::default()
        };
        let rc_state = Arc::new(Mutex::new(
            AppState::new(persistent, send, cc.egui_ctx.clone())
        ));

        Self {
            state: rc_state.clone(),
            current_screen: Self::make_screen(rc_state, ScreenType::Home),
            pipeline: Pipeline::new(),
            event_receiver: recv
        }
    }

    pub fn state(&self) -> Arc<Mutex<AppState>> {
        self.state.clone()
    }

    fn make_screen(state: Arc<Mutex<AppState>>, screen: ScreenType) -> Box<dyn Screen> {
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
                    let config = &self.state.lock().unwrap().persistent.config;
                    self.pipeline.start(config.clone()).unwrap();
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
        let guard = self.state.lock().unwrap();
        eframe::set_value(storage, eframe::APP_KEY, &guard.persistent);
    }
}