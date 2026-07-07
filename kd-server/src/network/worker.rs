use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use crate::encode::EncodeConfig;
use crate::network::NetworkConfig;
use crate::session::common::PacketQueue;
use crate::session::SessionWorker;
use crate::state::Client;

pub struct NetworkWorker {
    config: NetworkConfig,
    packet_queue: Arc<PacketQueue>,
    clients: Arc<Mutex<Vec<Client>>>,
    stopped: Arc<AtomicBool>
}

impl NetworkWorker {
    pub fn new(config: NetworkConfig, packet_queue: Arc<PacketQueue>, clients: Arc<Mutex<Vec<Client>>>, stopped: Arc<AtomicBool>) -> Self {
        Self {
            config,
            packet_queue,
            clients,
            stopped
        }
    }
}

impl SessionWorker for NetworkWorker {
    fn run(&mut self) {
        let video_port = self.config.video_port;
        let video_socket = UdpSocket::bind(("0.0.0.0", video_port))
            .expect(&format!("Failed to create video network socket at port {video_port}"));

        while !self.stopped.load(Ordering::SeqCst) {
            let Some(packet) = self.packet_queue.pop() else { break; };
            let encoded = packet.encode();
            for client in self.clients.lock().unwrap().iter() {
                let dest = SocketAddr::new(client.ip, video_port);
                if let Err(e) = video_socket.send_to(&encoded, dest) {
                    eprintln!("network: {e}");
                }
            }
        }
    }
}