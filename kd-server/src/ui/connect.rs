use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use eframe::emath::{Pos2, Rect, Vec2};
use egui::{Color32, Sense, Ui};
use qrcode::QrCode;
use uuid::Uuid;
use kd_shared::connect::ConnectParams;
use crate::http::SharedState;
use crate::network::HandshakeListener;
use crate::state::SessionState;
use crate::ui::AppEvent;
use crate::ui::screen::{Screen, ScreenType};

pub struct ConnectScreen {
    state: SharedState,
    connected: Arc<AtomicBool>,
    client_ip: Arc<Mutex<Option<String>>>,
}

impl ConnectScreen {
    pub fn new(state: SharedState) -> Self {
        Self {
            state,
            connected: Arc::new(AtomicBool::new(false)),
            client_ip: Arc::new(Mutex::new(None)),
        }
    }
}

impl Screen for ConnectScreen {
    fn on_show(&mut self) {
        let mut state = self.state.lock().unwrap();

        // Use token from /stream if already set, otherwise generate fresh (QR flow)
        let token = state.session
            .as_ref()
            .map(|s| s.token.clone())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let game_title = state.selected_game.clone().unwrap_or_default();

        // Store the partial session — client_ip filled in after handshake
        state.session = Some(SessionState {
            token: token.clone(),
            client_ip: String::new(),
            game_title,
        });

        let handshake_port = state.persistent.network.handshake_port;
        let connected = self.connected.clone();
        let client_ip = self.client_ip.clone();
        let ctx = state.ctx.clone();
        drop(state);

        std::thread::spawn(move || {
            let listener = HandshakeListener::new(handshake_port, token);
            loop {
                if let Ok(ip) = listener.listen() {
                    *client_ip.lock().unwrap() = Some(ip.to_string());
                    connected.store(true, Ordering::SeqCst);
                    ctx.request_repaint();
                    break;
                }
            }
        });
    }

    fn render(&mut self, ui: &mut Ui) {
        let mut state = self.state.lock().unwrap();

        ui.horizontal(|ui| {
            if ui.button("<- Back").clicked() {
                state.push_event(AppEvent::ScreenTransition(ScreenType::Home));
            }
            ui.heading("Connect");
        });

        if self.connected.load(Ordering::SeqCst) {
            if let Some(ip) = self.client_ip.lock().unwrap().clone() {
                // Fill in the client IP now that we have it
                if let Some(session) = state.session.as_mut() {
                    session.client_ip = ip;
                }
            }
            state.push_event(AppEvent::ScreenTransition(ScreenType::Session));
        }

        ui.separator();

        let ip = get_lan_ip().unwrap_or_else(|| "127.0.0.1".to_string());
        let game_title = state.selected_game.clone().unwrap_or_default();
        let game = state.persistent.games.iter()
            .find(|g| g.metadata.title == game_title)
            .unwrap();

        ui.label(format!("Game:    {}", game.metadata.title));
        ui.label(format!("Address: {}:{}", ip, state.persistent.network.video_port));
        let token = state.session.as_ref().map(|s| s.token.clone()).unwrap_or_default();
        ui.label(format!("Session: {}", &token[..8.min(token.len())]));

        ui.add_space(16.0);
        ui.label("Scan with Kodomo on your iPhone:");
        ui.add_space(8.0);

        let params = ConnectParams::new(
            ip.clone(),
            state.persistent.network.video_port,
            token.clone(),
            game.metadata.title.clone(),
            state.persistent.network.handshake_port,
            state.persistent.network.http_port,
            state.persistent.network.input_port,
        );
        let url = params.to_url();

        match QrCode::new(url.as_bytes()) {
            Ok(code) => {
                draw_qr_code(ui, &code, 7.0);
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