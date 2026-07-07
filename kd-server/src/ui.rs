mod games;
mod connect;
mod screen;
mod session;
mod sidebar;

use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use crate::http::SharedState;
use crate::session::Session;
use crate::state::{AppState, Client, Game, PersistentState};
use crate::ui::connect::ConnectScreen;
use crate::ui::games::GamesScreen;
use crate::ui::screen::{Screen, ScreenType};
use crate::ui::session::SessionScreen;
use crate::ui::sidebar::Sidebar;

pub enum AppEvent {
    ClientConnected(Client),
    StartSession(Game),
    EndSession,
    ScreenTransition(ScreenType)
}

pub struct ServerApp {
    state: SharedState,
    event_receiver: crossbeam_channel::Receiver<AppEvent>,
    sidebar: Sidebar,
    screens: Vec<Box<dyn Screen>>,
    current_screen: ScreenType,
}

impl ServerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (send, recv) = crossbeam_channel::unbounded();
        let persistent = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            PersistentState::default()
        };
        let state = Arc::new(Mutex::new(AppState::new(persistent, send, cc.egui_ctx.clone())));

        let screens: Vec<Box<dyn Screen>> = vec![
            Box::new(GamesScreen::new(state.clone())),
            Box::new(ConnectScreen::new(state.clone())),
            Box::new(SessionScreen::new(state.clone())),
        ];

        Self {
            state: state.clone(),
            event_receiver: recv,
            sidebar: Sidebar::new(state.clone()),
            screens,
            current_screen: ScreenType::Games
        }
    }

    pub fn state(&self) -> Arc<Mutex<AppState>> {
        self.state.clone()
    }

    fn process_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.event_receiver.try_recv() {
            let mut state = self.state.lock().unwrap();
            match event {
                AppEvent::ScreenTransition(screen) => {
                    drop(state);
                    if self.current_screen != screen {
                        self.current_screen = screen;
                        self.screens[self.current_screen as usize].on_show();
                    }
                }
                AppEvent::ClientConnected(client) => {
                    state.add_client(client);
                }
                AppEvent::StartSession(game) => {
                    state.start_session(game);
                }
                AppEvent::EndSession => {
                    state.stop_session();
                }
            }

            ctx.request_repaint();
        }
    }
}

impl eframe::App for ServerApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.process_events(ui.ctx());

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.sidebar.render(ui);
            self.screens[self.current_screen as usize].render(ui);
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.state.lock().unwrap().persistent);
    }
}
