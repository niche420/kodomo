use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use crate::ui::{ServerApp};

mod capture;
mod encode;
mod pipeline;
mod network;
mod ui;
mod http;
mod profile;
mod state;

fn main() -> anyhow::Result<()> {
    let mut native_options = eframe::NativeOptions::default();
    native_options.persistence_path = Some(PathBuf::from("kd-server-data"));
    native_options.persist_window = true;
    eframe::run_native("Kodomo-Server", native_options, Box::new(|cc| Ok(Box::new(ServerApp::new(cc)))))?;
    
    Ok(())
}
