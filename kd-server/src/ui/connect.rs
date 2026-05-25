use std::cell::RefCell;
use std::net::UdpSocket;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use eframe::emath::{Pos2, Rect, Vec2};
use egui::{Color32, Sense, Ui};
use qrcode::QrCode;
use uuid::Uuid;
use kd_shared::connect::ConnectParams;
use crate::network::HandshakeListener;
use crate::ui::AppState;
use crate::ui::screen::{Screen, ScreenType};

pub struct ConnectScreen {
    state: Rc<RefCell<AppState>>,
    connected: Arc<AtomicBool>,
    client_ip: Arc<Mutex<Option<String>>>
}

impl ConnectScreen {
    pub fn new(state: Rc<RefCell<AppState>>) -> ConnectScreen {
        Self {
            state,
            connected: Arc::new(AtomicBool::new(false)),
            client_ip: Arc::new(Mutex::new(None)),
        }
    }
}

impl Screen for ConnectScreen {
    fn on_show(&mut self) {
        let session = Uuid::new_v4().to_string();
        self.state.borrow_mut().session = Some(session.clone());
        let handshake_port = self.state.borrow().config.network.handshake_port;
        let connected = self.connected.clone();
        let client_ip = self.client_ip.clone();

        std::thread::spawn(move || {
            let listener = HandshakeListener::new(handshake_port, session);
            if let Ok(ip) = listener.listen() {
                *client_ip.lock().unwrap() = Some(ip.to_string());
                connected.store(true, Ordering::SeqCst);
            }
        });
    }

    fn render(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button("<- Back").clicked() {
                self.state.borrow_mut().transition_to(ScreenType::Home);
            }
            ui.heading("Connect");
        });

        if self.connected.load(Ordering::SeqCst) {
            if let Some(ip) = self.client_ip.lock().unwrap().clone() {
                self.state.borrow_mut().config.network.dest_ip = ip;
            }
            self.state.borrow_mut().transition_to(ScreenType::Session);
        }

        ui.separator();

        let ip = get_lan_ip().unwrap_or_else(|| "127.0.0.1".to_string());

        let state = self.state.borrow();
        let game = state.games.iter().find(|g| g.metadata.title == state.selected_game.clone().unwrap_or(String::new())).unwrap();
        ui.label(format!("Game:    {}", game.metadata.title));
        ui.label(format!("Address: {}:{}", ip, state.config.network.video_port));
        let session = state.session.as_ref().unwrap();
        ui.label(format!("Session: {}", &session[..8])); // show prefix only

        ui.add_space(16.0);
        ui.label("Scan with Kodomo on your iPhone:");
        ui.add_space(8.0);
        let params = ConnectParams::new(ip.clone(), state.config.network.video_port, session.to_string(),
                                        game.metadata.title.clone(), state.config.network.handshake_port);
        let url = params.to_url();

        match QrCode::new(url.as_bytes()) {
            Ok(code) => {
                draw_qr_code(ui, &code, 7.0);
                ui.add_space(8.0);
                // Small readable URL beneath the QR code
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

/// Renders a QR code using egui's immediate-mode painter.
/// `cell_size` is the pixel size of each module (dark/light square).
fn draw_qr_code(ui: &mut egui::Ui, code: &QrCode, cell_size: f32) {
    let modules = code.width();

    // Add a quiet-zone border of 4 modules on each side (QR spec minimum).
    const QUIET: usize = 4;
    let total_cells = modules + QUIET * 2;
    let total_px = total_cells as f32 * cell_size;

    let (response, painter) = ui.allocate_painter(Vec2::splat(total_px), Sense::hover());
    let origin = response.rect.min;

    // White background including the quiet zone
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

/// Determines the LAN IP by routing toward an external address without
/// sending any traffic — works on any interface including Wi-Fi.
fn get_lan_ip() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    Some(socket.local_addr().ok()?.ip().to_string())
}