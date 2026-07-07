pub mod common;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use crate::capture::worker::CaptureWorker;
use crate::encode::EncodeConfig;
use crate::encode::worker::EncodeWorker;
use crate::input::worker::InputWorker;
use crate::network::NetworkConfig;
use crate::network::worker::NetworkWorker;
use crate::session::common::{FrameSlot, PacketQueue};
use crate::state::{Client, Game};

pub trait SessionWorker: Send + Sync {
    fn run(&mut self);
}

pub struct Session {
    pub game: Game,
    pub clients: Arc<Mutex<Vec<Client>>>,
    network_config: NetworkConfig,
    stopped: Arc<AtomicBool>,
    worker_threads: Vec<JoinHandle<()>>,
    packet_queue: Arc<PacketQueue>
}

impl Session {
    pub fn start(game: Game, clients: Vec<Client>, encode_config: EncodeConfig, network_config: NetworkConfig) -> Session {
        let frame_slot = Arc::new(FrameSlot::default());
        let packet_queue = Arc::new(PacketQueue::new());
        let exe_path = game.exe_path.clone();
        let clients = Arc::new(Mutex::new(clients));
        let stopped = Arc::new(AtomicBool::new(false));

        let mut session = Session {
            game,
            clients: clients.clone(),
            network_config: network_config.clone(),
            stopped: stopped.clone(),
            worker_threads: Vec::new(),
            packet_queue: packet_queue.clone(),
        };

        session.spawn_worker(Box::new(CaptureWorker::new(exe_path, frame_slot.clone(), stopped.clone())));
        session.spawn_worker(Box::new(EncodeWorker::new(encode_config, frame_slot, packet_queue.clone(), stopped.clone())));
        session.spawn_worker(Box::new(NetworkWorker::new(network_config.clone(), packet_queue.clone(), clients.clone(), stopped.clone())));
        session.spawn_worker(Box::new(InputWorker::new(network_config, clients, stopped.clone())));

        session
    }

    pub fn add_client(&mut self, client: Client) {
        self.clients.lock().unwrap().push(client);
        self.spawn_worker(Box::new(InputWorker::new(self.network_config.clone(), self.clients.clone(), self.stopped.clone())));
    }

    pub fn num_clients(&self) -> usize {
        self.clients.lock().unwrap().len()
    }

    fn spawn_worker(&mut self, mut worker: Box<dyn SessionWorker>) {
        self.worker_threads.push(std::thread::spawn(move || worker.run()));
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.packet_queue.reset();
        self.worker_threads.drain(..).for_each(|t| { t.join().ok(); });
    }
}