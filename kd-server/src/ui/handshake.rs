use std::net::UdpSocket;
use std::path::{Path, PathBuf};
use std::any::Any;
use std::sync::{Arc, Mutex};
use eframe::emath::{Pos2, Rect, Vec2};
use egui::{Color32, Sense, Ui};
use qrcode::QrCode;
use uuid::Uuid;
use kd_shared::connect::ConnectParams;
use crate::http::SharedState;
use crate::network::HandshakeListener;
use crate::profile::load_profile;
use crate::state::ClientSession;
use crate::ui::AppEvent;
use crate::ui::screen::{Screen, ScreenType};

pub struct HandshakeScreen {
    state: SharedState,
    game_title: String,
    exe_path: PathBuf,
    token: String,
}

impl HandshakeScreen {
    pub fn new(
        state: SharedState,
        game_title: String,
        exe_path: PathBuf,
        token: Option<String>,
    ) -> Self {
        let token = token.unwrap_or_else(|| Uuid::new_v4().to_string());

        let (handshake_port, ctx) = {
            let s = state.lock().unwrap();
            (s.persistent.network.handshake_port, s.ctx.clone())
        };

        let token_clone = token.clone();
        let game_title_clone = game_title.clone();
        let state_clone = state.clone();

        std::thread::spawn(move || {
            loop {
                let listener = HandshakeListener::new(handshake_port, token_clone.clone());
                match listener.listen() {
                    Ok(result) => {
                        let profile = {
                            let s = state_clone.lock().unwrap();
                            s.persistent.games
                                .iter()
                                .find(|g| g.metadata.title == game_title_clone)
                                .and_then(|g| g.active_profile.as_deref())
                                .and_then(|name| load_profile(&game_title_clone, name))
                        };
                        state_clone.lock().unwrap().push_event(AppEvent::ClientConnected(
                            ClientSession {
                                ip: result.client_ip.to_string(),
                                profile,
                                input_socket: result.input_socket,
                            }
                        ));
                        ctx.request_repaint();
                    }
                    Err(e) => eprintln!("handshake: listener error: {e}"),
                }
            }
        });

        Self { state, game_title, exe_path, token }
    }

    pub fn game_title(&self) -> &str { &self.game_title }
    pub fn exe_path(&self) -> &Path  { &self.exe_path }
}

impl Screen for HandshakeScreen {
    fn render(&mut self, ui: &mut Ui) {
        let mut state = self.state.lock().unwrap();

        ui.horizontal(|ui| {
            if ui.button("<- Back").clicked() {
                state.push_event(AppEvent::ScreenTransition(ScreenType::Home));
            }
            ui.heading("Waiting for connection");
        });

        ui.separator();

        let ip = get_lan_ip().unwrap_or_else(|| "127.0.0.1".to_string());
        ui.label(format!("Game:    {}", self.game_title));
        ui.label(format!("Address: {}:{}", ip, state.persistent.network.video_port));
        ui.label(format!("Session: {}", &self.token[..8.min(self.token.len())]));

        ui.add_space(16.0);
        ui.label("Scan with Kodomo on your iPhone:");
        ui.add_space(8.0);

        let params = ConnectParams::new(
            ip,
            state.persistent.network.video_port,
            self.token.clone(),
            self.game_title.clone(),
            state.persistent.network.handshake_port,
            state.persistent.network.http_port,
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

    fn get_type(&self) -> ScreenType { ScreenType::Handshake }
    fn as_any(&self) -> &dyn Any { self }
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