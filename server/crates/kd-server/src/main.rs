use clap::Parser;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{error, info, warn};

mod config;
mod server;
mod metrics;

use config::Config;
use server::StreamingServer;

#[derive(Parser, Debug)]
#[command(name = "streaming-server")]
#[command(about = "High-performance game streaming server", long_about = None)]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// Monitor index to capture (overrides config)
    #[arg(short, long)]
    monitor: Option<u32>,

    /// Server port (overrides config)
    #[arg(short, long)]
    port: Option<u16>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// List available encoders and exit
    #[arg(long)]
    list_encoders: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level))
        )
        .with_target(false)
        .init();

    info!("🎮 Game Streaming Server v{}", env!("CARGO_PKG_VERSION"));

    // List encoders if requested
    if args.list_encoders {
        list_available_encoders();
        return Ok(());
    }

    // Load configuration
    let mut config = if args.config.exists() {
        info!("Loading configuration from: {}", args.config.display());
        Config::from_file(&args.config)?
    } else {
        warn!("Config file not found, using defaults");
        Config::default()
    };

    // Apply CLI overrides
    if let Some(port) = args.port {
        config.network.port = port;
    }

    // Validate configuration
    config.validate()?;

    info!("Configuration:");
    info!("  Video: {}x{} @ {} fps, {} kbps", 
          config.video.width, config.video.height, 
          config.video.fps, config.video.bitrate_kbps);
    info!("  Codec: {:?} (HW accel: {})", 
          config.video.codec, config.video.hw_accel);
    info!("  Network: {:?} on port {}", 
          config.network.transport, config.network.port);

    // Create and run server
    let mut server = StreamingServer::new(config)?;

    // Handle Ctrl+C gracefully
    let shutdown_signal = tokio::spawn(async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        info!("Received Ctrl+C, shutting down...");
    });

    // Start server
    server.start().await?;

    // Wait for shutdown signal
    shutdown_signal.await?;

    // Stop server
    server.stop().await?;

    info!("Server stopped gracefully");
    Ok(())
}

fn list_available_encoders() {
    println!("Available video encoders:");
    let encoders = kd_encoder::EncoderFactory::list_available_encoders();
    for (i, encoder) in encoders.iter().enumerate() {
        println!("  {}. {}", i + 1, encoder);
    }
}