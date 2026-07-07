use std::path::PathBuf;
use std::sync::{Arc, Mutex};

mod capture;
mod encode;
mod network;
mod ui;
mod http;
mod profile;
mod state;
mod input;
mod session;

fn main() -> anyhow::Result<()> {
    // Spawn HTTP server on a background thread before eframe starts.
    // ServerApp::new will hand it the real shared state once eframe
    // gives us a CreationContext. We pass the Arc in via the closure.
    //
    // The channel used here is a placeholder — ServerApp::new installs
    // the real sender into AppState. HTTP handlers that need to push
    // events (like /stream) will use whatever sender is in AppState at
    // call time, which by then is the real one.
    let (http_tx, http_rx) = tokio::sync::oneshot::channel::<http::SharedState>();

    std::thread::spawn(move || {
        tokio::runtime::Runtime::new()
            .expect("tokio runtime")
            .block_on(async {
                // Wait for the UI thread to hand us the shared state
                let shared = http_rx.await.expect("shared state channel closed");
                http::serve(shared).await;
            });
    });

    let mut native_options = eframe::NativeOptions::default();
    native_options.persistence_path = Some(PathBuf::from("kd-server-data"));
    native_options.persist_window = true;

    eframe::run_native(
        "Kodomo Server",
        native_options,
        Box::new(move |cc| {
            let app = ui::ServerApp::new(cc);
            // Send the shared state to the HTTP thread
            let _ = http_tx.send(app.state());
            Ok(Box::new(app))
        }),
    )?;

    Ok(())
}