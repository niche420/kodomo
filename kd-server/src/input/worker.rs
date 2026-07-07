use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use kd_shared::profile::InputEvent;
use crate::network::NetworkConfig;
use crate::session::SessionWorker;
use crate::state::Client;

pub struct InputWorker {
    network_config: NetworkConfig,
    clients: Arc<Mutex<Vec<Client>>>,
    stopped: Arc<AtomicBool>,
}

impl InputWorker {
    pub fn new(network_config: NetworkConfig, clients: Arc<Mutex<Vec<Client>>>, stopped: Arc<AtomicBool>) -> Self {
        Self { network_config, clients,stopped }
    }
}

impl SessionWorker for InputWorker {
    fn run(&mut self) {
        let input_port = self.network_config.input_port;
        let input_socket = UdpSocket::bind(
            ("0.0.0.0", input_port)
        ).expect(&format!("Failed to create input socket at port {input_port}"));
        let mut injector = crate::input::create_injector();
        let mut buf = [0u8; 4096];

        while !self.stopped.load(Ordering::SeqCst) {
            match input_socket.recv_from(&mut buf) {
                Ok((len, sender)) => {
                    if let Some(client) =
                        self.clients.lock().unwrap().iter().find(|c| c.ip == sender.ip())
                    {
                        if let Some(profile) = &client.profile {
                            if let Ok(event) =
                                serde_json::from_slice::<InputEvent>(&buf[..len])
                            {
                                crate::input::dispatch(
                                    &mut *injector,
                                    &event,
                                    profile,
                                );
                            }
                        }
                    }
                }

                Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
                    {
                        continue;
                    }

                Err(e) => eprintln!("input: {e}"),
            }
        }
    }
}