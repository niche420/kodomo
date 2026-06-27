mod home;
mod handshake;
pub(crate) mod screen;
mod session;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use eframe::App;
use crate::pipeline::Pipeline;
use crate::state::{AppState, ClientSession, PersistentState, SessionState};
use crate::ui::handshake::HandshakeScreen;
use crate::ui::home::HomeScreen;
use crate::ui::screen::{Screen, ScreenType};
use crate::ui::session::SessionScreen;

pub enum AppEvent {
    NavigateToConnect { game_title: String, exe_path: PathBuf, token: Option<String> },
    ClientConnected(ClientSession),
    ScreenTransition(ScreenType),
    PipelineEnd,
}

pub struct ServerApp {
    state: Arc<Mutex<AppState>>,
    current_screen: Box<dyn Screen>,
    pipeline: Pipeline,
    event_receiver: crossbeam_channel::Receiver<AppEvent>,
    /// Stash for the current handshake — needed to build SessionState when
    /// the first client connects. Cleared once the pipeline starts.
    pending_game: Option<(String, PathBuf)>,
}

impl ServerApp {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (send, recv) = crossbeam_channel::unbounded();
        let persistent = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            PersistentState::default()
        };
        let state = Arc::new(Mutex::new(AppState::new(persistent, send, cc.egui_ctx.clone())));

        Self {
            state: state.clone(),
            current_screen: Box::new(HomeScreen::new(state)),
            pipeline: Pipeline::new(),
            event_receiver: recv,
            pending_game: None,
        }
    }

    pub fn state(&self) -> Arc<Mutex<AppState>> {
        self.state.clone()
    }

    fn process_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.event_receiver.try_recv() {
            match event {
                AppEvent::NavigateToConnect { game_title, exe_path, token } => {
                    self.pending_game = Some((game_title.clone(), exe_path.clone()));
                    self.current_screen = Box::new(
                        HandshakeScreen::new(self.state.clone(), game_title, exe_path, token)
                    );
                    ctx.request_repaint();
                }

                AppEvent::ClientConnected(client) => {
                    let mut state = self.state.lock().unwrap();
                    match state.session.as_mut() {
                        Some(session) => {
                            // Additional client joining an active stream
                            session.clients.push(ClientSession {
                                ip: client.ip.clone(),
                                profile: client.profile.clone(),
                                input_socket: client.input_socket.try_clone()
                                    .expect("failed to clone input socket"),
                            });
                            drop(state);
                            self.pipeline.add_client(client);
                        }
                        None => {
                            // First client — start pipeline
                            let (game_title, exe_path) = self.pending_game.take()
                                .expect("ClientConnected with no pending game");

                            let session = SessionState {
                                game_title,
                                exe_path,
                                clients: vec![client],
                            };
                            state.session = Some(SessionState {
                                game_title: session.game_title.clone(),
                                exe_path: session.exe_path.clone(),
                                clients: vec![],
                            });
                            drop(state);

                            self.pipeline.start(
                                &self.state.lock().unwrap().persistent.encode,
                                &self.state.lock().unwrap().persistent.network,
                                session,
                            ).ok();

                            self.current_screen = Box::new(SessionScreen::new(self.state.clone()));
                            ctx.request_repaint();
                        }
                    }
                }

                AppEvent::ScreenTransition(next) => {
                    self.current_screen = match next {
                        ScreenType::Home    => Box::new(HomeScreen::new(self.state.clone())),
                        ScreenType::Session => Box::new(SessionScreen::new(self.state.clone())),
                        ScreenType::Handshake => panic!("use NavigateToConnect"),
                    };
                    ctx.request_repaint();
                }

                AppEvent::PipelineEnd => {
                    self.pipeline.stop();
                    self.state.lock().unwrap().session = None;
                    self.pending_game = None;
                    self.current_screen = Box::new(HomeScreen::new(self.state.clone()));
                    ctx.request_repaint();
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
        eframe::set_value(storage, eframe::APP_KEY, &self.state.lock().unwrap().persistent);
    }
}