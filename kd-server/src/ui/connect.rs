use std::any::Any;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, UdpSocket};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use eframe::emath::{Pos2, Rect, Vec2};
use egui::{Color32, Sense, Ui};
use qrcode::QrCode;
use uuid::Uuid;
use kd_shared::connect::ConnectParams;
use crate::http::SharedState;
use crate::state::Client;
use crate::ui::AppEvent;
use crate::ui::screen::{Screen, ScreenType};

pub struct ConnectScreen {
    state: SharedState,
}

impl ConnectScreen {
    pub fn new(state: SharedState) -> ConnectScreen {
        Self { state }
    }
}

impl Screen for ConnectScreen {
    fn render(&mut self, ui: &mut Ui) {
        let mut state = self.state.lock().unwrap();

        // Lazily ensure a token exists so this screen has something to show
        // on first render; if the /stream HTTP flow already minted one
        // (e.g. an iOS client just started a stream), that one is reused
        // here so the displayed QR always reflects the live token.
        let token = state.current_token.unwrap_or_else(|| state.new_token());

        ui.heading("Connect");
        ui.separator();

        // Connection info
        let ip = get_lan_ip().unwrap_or_else(|| "127.0.0.1".to_string());
        ui.label(format!("Address: {}", ip));
        ui.label(format!("Session: {}", &token.to_string()));
        ui.add_space(8.0);

        // Connected clients
        let num_clients = if let Some(session) = &state.session {
            session.clients.lock().unwrap().len()
        } else {
            state.pending_clients.len()
        };

        if num_clients == 0 {
            ui.label("No clients connected.");
        } else {
            ui.label(format!("{} client(s) connected:", num_clients));
            for client in &state.pending_clients {
                ui.horizontal(|ui| {
                    ui.colored_label(Color32::YELLOW, "●");
                    ui.label(format!("{} (waiting)", client.ip));
                });
            }
            if let Some(session) = state.session.as_ref() {
                for client in session.clients.lock().unwrap().iter() {
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::GREEN, "●");
                        ui.label(format!("{} (streaming)", client.ip));
                    });
                }
            }
        }

        ui.add_space(16.0);
        ui.label("Scan to connect:");
        ui.add_space(8.0);

        let params = ConnectParams::new(
            ip,
            state.persistent.network.video_port,
            token.to_string(),
            state.persistent.network.handshake_port,
            state.persistent.network.http_port,
        );
        let url = params.to_url();
        drop(state);

        match QrCode::new(url.as_bytes()) {
            Ok(code) => {
                draw_qr_code(ui, &code, 6.0);
                ui.add_space(8.0);
                ui.label(egui::RichText::new(&url).small().weak().monospace());
            }
            Err(e) => {
                ui.colored_label(Color32::RED, format!("QR error: {e}"));
            }
        }
    }

    fn get_type(&self) -> ScreenType {
        ScreenType::Connect
    }
}



fn draw_qr_code(ui: &mut egui::Ui, code: &QrCode, cell_size: f32) {
    let modules = code.width();
    const QUIET: usize = 4;
    let total_px = (modules + QUIET * 2) as f32 * cell_size;
    let (response, painter) = ui.allocate_painter(Vec2::splat(total_px), Sense::hover());
    let origin = response.rect.min;
    painter.rect_filled(response.rect, 4.0, Color32::WHITE);
    let offset = QUIET as f32 * cell_size;
    for row in 0..modules {
        for col in 0..modules {
            if code[(row, col)] == qrcode::Color::Dark {
                let x = origin.x + offset + col as f32 * cell_size;
                let y = origin.y + offset + row as f32 * cell_size;
                painter.rect_filled(
                    Rect::from_min_size(Pos2::new(x, y), Vec2::splat(cell_size)),
                    0.0,
                    Color32::BLACK,
                );
            }
        }
    }
}

fn get_lan_ip() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    Some(socket.local_addr().ok()?.ip().to_string())
}

/// Spawns the single, long-lived handshake TCP listener for the server.
/// Call this exactly once at startup. Whichever flow wants to accept a
/// new client next (the QR Connect screen, or the HTTP /stream endpoint)
/// mints a fresh token via `AppState::new_token` and stores it in shared
/// state; this listener always checks incoming handshakes against
/// whatever token is current at the moment the connection arrives.
pub fn spawn_handshake_listener(state: SharedState) {
    let handshake_port = match state.lock() {
        Ok(state) => state.persistent.network.handshake_port,
        Err(_) => { eprintln!("Failed to lock on state"); return; }
    };
    let state_clone = state.clone();

    std::thread::spawn(move || {
        let listener = match TcpListener::bind(format!("0.0.0.0:{}", handshake_port)) {
            Ok(l) => l,
            Err(e) => { eprintln!("handshake: failed to bind: {e}"); return; }
        };
        eprintln!("handshake: listening on port {}", handshake_port);

        loop {
            match listener.accept() {
                Ok((mut stream, addr)) => {

                    // Handle each connection on its own thread so the
                    // listener can immediately accept the next one
                    let state_clone = state_clone.clone();
                    std::thread::spawn(move || {
                        let reader_stream = match stream.try_clone() {
                            Ok(s) => s,
                            Err(e) => { eprintln!("handshake: clone error: {e}"); return; }
                        };
                        let mut reader = BufReader::new(reader_stream);

                        let mut received_token_str = String::new();
                        if reader.read_line(&mut received_token_str).is_err() { return; }

                        let input_port = {
                            let current_token = state_clone.lock().unwrap().current_token;
                            match Uuid::from_str(received_token_str.trim()) {
                                Ok(received_token) if Some(received_token) == current_token => {
                                    state_clone.lock().unwrap().persistent.network.input_port
                                }
                                _ => {
                                    let _ = stream.write_all(b"err\n");
                                    eprintln!("handshake: token mismatch from {}", addr.ip());
                                    return;
                                }
                            }
                        };

                        if stream.write_all(format!("ok:{}\n", input_port).as_bytes()).is_err() {
                            return;
                        }

                        let mut ready = String::new();
                        if reader.read_line(&mut ready).is_err() { return; }
                        if ready.trim() != "ready" { return; }

                        let ip = addr.ip();
                        eprintln!("handshake: client connected: {}", ip);

                        let mut state = state_clone.lock().unwrap();
                        state.push_event(AppEvent::ClientConnected(Client {
                            // The port is unknown until the client sends its first UDP packet.
                            ip: addr.ip(),
                            profile: None,
                        }));
                    });
                }
                Err(e) => eprintln!("handshake: accept error: {e}"),
            }
        }
    });
}