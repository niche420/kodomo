use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::path::PathBuf;
use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use kd_shared::game::GameMetadata;
use kd_shared::profile::GameProfile;
use crate::encode::EncodeConfig;
use crate::network::NetworkConfig;
use crate::session::Session;
use crate::ui::AppEvent;

#[derive(Serialize, Deserialize, Default)]
pub struct PersistentState {
    pub games: Vec<Game>,
    pub network: NetworkConfig,
    pub encode: EncodeConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Game {
    pub metadata: GameMetadata,
    pub exe_path: PathBuf,
    pub thumbnail: Option<PathBuf>,
    pub active_profile: Option<String>,
    pub is_running: bool,
}

#[derive(Debug, Clone)]
pub struct Client {
    pub ip: IpAddr,
    pub profile: Option<GameProfile>,
}

pub struct AppState {
    pub persistent: PersistentState,
    pub pending_clients: Vec<Client>,
    pub ctx: egui::Context,
    event_sender: Sender<AppEvent>,
    pub(crate) session: Option<Session>,
}

impl AppState {
    pub fn new(
        persistent: PersistentState,
        event_sender: Sender<AppEvent>,
        ctx: egui::Context,
    ) -> Self {
        Self {
            persistent,
            pending_clients: Vec::new(),
            ctx,
            event_sender,
            session: None,
        }
    }

    pub fn push_event(&mut self, event: AppEvent) {
        self.event_sender.send(event).unwrap();
        self.ctx.request_repaint();
    }

    pub fn game(&self, title: &str) -> Option<&Game> {
        self.persistent.games.iter().find(|g| g.metadata.title == title)
    }

    pub fn game_mut(&mut self, title: &str) -> Option<&mut Game> {
        self.persistent.games.iter_mut().find(|g| g.metadata.title == title)
    }
    
    pub fn add_client(&mut self, client: Client) {
        if let Some(session) = self.session.as_mut() {
            session.add_client(client);
        } else {
            self.pending_clients.push(client);
        }
    }
    
    pub fn start_session(&mut self, game: Game) {
        if self.session.is_some() {
            eprintln!("Cannot start a new session when previous one in progress");
        } else {
            let network = self.persistent.network.clone();
            let encode = self.persistent.encode.clone();
            let clients = self.pending_clients.drain(..).collect();
            self.session = Some(Session::start(game, clients, encode, network));
        }
    }
    
    pub fn stop_session(&mut self) {
        if self.is_streaming(){
            self.session.take();
        } else {
            eprintln!("Cannot stop a session when there is no session");
        }
    }
    
    pub fn is_streaming(&self) -> bool { self.session.is_some() }
}