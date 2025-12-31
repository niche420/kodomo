use crate::config::Config;
use crate::metrics::MetricsCollector;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

use kd_capture::{CaptureConfig, CapturedFrame, ScreenCapture, ScreenCaptureManager};
use kd_encoder::{EncoderConfig, EncoderFactory, RawFrame, VideoEncoder, PixelFormat};
use kd_network::{NetworkConfig, NetworkTransport, TransportFactory, Packet, PacketType};
use kd_input::{InputHandler, InputEvent, InputConfig};
use bytes::Bytes;

const FRAME_CHANNEL_SIZE: usize = 8;
const PACKET_CHANNEL_SIZE: usize = 32;

pub struct StreamingServer {
    config: Config,
    metrics: Arc<RwLock<MetricsCollector>>,
    shutdown_tx: broadcast::Sender<()>,

    // Channels for pipeline
    frame_tx: mpsc::Sender<CapturedFrame>,
    frame_rx: Option<mpsc::Receiver<CapturedFrame>>,

    packet_tx: mpsc::Sender<EncodedPacketWithMeta>,
    packet_rx: Option<mpsc::Receiver<EncodedPacketWithMeta>>,
}

struct EncodedPacketWithMeta {
    data: Bytes,
    is_keyframe: bool,
    frame_number: u64,
}

impl StreamingServer {
    pub fn new(config: Config) -> Result<Self> {
        let (shutdown_tx, _) = broadcast::channel(1);
        let (frame_tx, frame_rx) = mpsc::channel(FRAME_CHANNEL_SIZE);
        let (packet_tx, packet_rx) = mpsc::channel(PACKET_CHANNEL_SIZE);

        Ok(Self {
            config,
            metrics: Arc::new(RwLock::new(MetricsCollector::new())),
            shutdown_tx,
            frame_tx,
            frame_rx: Some(frame_rx),
            packet_tx,
            packet_rx: Some(packet_rx),
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting streaming server...");

        // Initialize and start all components
        self.start_capture_loop().await?;
        self.start_encoder_loop().await?;
        self.start_network_loop().await?;
        self.start_input_loop().await?;
        self.start_metrics_loop().await?;

        info!("🚀 Streaming server is running!");
        info!("   Listening on {}:{}",
              self.config.network.bind_address,
              self.config.network.port);
        info!("   Press Ctrl+C to stop");

        Ok(())
    }

    async fn start_capture_loop(&mut self) -> Result<()> {
        info!("Initializing screen capture...");

        // Create and initialize capture
        let mut capture = ScreenCaptureManager::new()
            .map_err(|e| anyhow::anyhow!("Capture init failed: {}", e))?;

        let capture_config = CaptureConfig {
            monitor_index: self.config.capture.monitor_index,
            width: self.config.video.width,
            height: self.config.video.height,
            fps: self.config.video.fps,
        };

        capture.init(capture_config)
            .map_err(|e| anyhow::anyhow!("Capture config failed: {}", e))?;

        info!("✓ Screen capture initialized: {}x{} @ {} FPS",
              self.config.video.width,
              self.config.video.height,
              self.config.video.fps);

        // Spawn capture task
        let frame_tx = self.frame_tx.clone();
        let metrics = self.metrics.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let fps = self.config.video.fps;

        tokio::spawn(async move {
            let frame_interval = Duration::from_micros(1_000_000 / fps as u64);
            let mut interval = interval(frame_interval);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            let mut frame_number = 0u64;
            let mut consecutive_errors = 0u32;

            info!("Capture loop started");

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        match capture.capture_frame() {
                            Ok(frame) => {
                                consecutive_errors = 0;
                                frame_number += 1;

                                // Update metrics
                                {
                                    let mut m = metrics.write().await;
                                    m.frames_captured += 1;
                                }

                                // Send frame to encoder
                                if let Err(e) = frame_tx.send(frame).await {
                                    error!("Failed to send frame to encoder: {}", e);
                                    break;
                                }

                                if frame_number % 60 == 0 {
                                    debug!("Captured {} frames", frame_number);
                                }
                            }
                            Err(e) => {
                                // NoFrame is normal when nothing changed
                                if !matches!(e, kd_capture::CaptureError::NoFrame) {
                                    consecutive_errors += 1;
                                    warn!("Capture error ({}): {}", consecutive_errors, e);

                                    if consecutive_errors > 100 {
                                        error!("Too many consecutive capture errors, stopping");
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Capture loop shutting down");
                        break;
                    }
                }
            }

            let _ = capture.shutdown();
            info!("Capture loop stopped");
        });

        Ok(())
    }

    async fn start_encoder_loop(&mut self) -> Result<()> {
        info!("Initializing video encoder...");

        // Create encoder
        let encoder_config = EncoderConfig {
            width: self.config.video.width,
            height: self.config.video.height,
            fps: self.config.video.fps,
            bitrate_kbps: self.config.video.bitrate_kbps,
            codec: match self.config.video.codec {
                crate::config::Codec::H264 => kd_encoder::VideoCodec::H264,
                crate::config::Codec::H265 => kd_encoder::VideoCodec::H265,
                crate::config::Codec::VP9 => kd_encoder::VideoCodec::VP9,
            },
            preset: kd_encoder::EncoderPreset::Fast,
            keyframe_interval: self.config.video.keyframe_interval,
            use_hardware: self.config.video.hw_accel,
        };

        let mut encoder = EncoderFactory::create(encoder_config)
            .map_err(|e| anyhow::anyhow!("Encoder init failed: {}", e))?;

        encoder.init(encoder.get_config().clone())
            .map_err(|e| anyhow::anyhow!("Encoder config failed: {}", e))?;

        info!("✓ Video encoder initialized: {:?}, HW accel: {}",
              self.config.video.codec,
              self.config.video.hw_accel);

        // Spawn encoder task
        let mut frame_rx = self.frame_rx.take().unwrap();
        let packet_tx = self.packet_tx.clone();
        let metrics = self.metrics.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            info!("Encoder loop started");
            let mut frame_number = 0u64;

            loop {
                tokio::select! {
                    Some(captured_frame) = frame_rx.recv() => {
                        frame_number += 1;

                        // Convert to RawFrame
                        let raw_frame = RawFrame {
                            data: captured_frame.data,
                            width: captured_frame.width,
                            height: captured_frame.height,
                            stride: captured_frame.stride,
                            format: match captured_frame.format {
                                kd_capture::PixelFormat::BGRA => PixelFormat::BGRA,
                                kd_capture::PixelFormat::RGBA => PixelFormat::RGBA,
                                kd_capture::PixelFormat::NV12 => PixelFormat::NV12,
                            },
                            pts: frame_number,
                            timestamp: captured_frame.timestamp,
                        };

                        // Encode frame
                        match encoder.encode(&raw_frame) {
                            Ok(Some(packet)) => {
                                // Update metrics
                                {
                                    let mut m = metrics.write().await;
                                    m.frames_encoded += 1;
                                    m.bytes_encoded += packet.data.len() as u64;
                                }

                                // Send to network
                                let meta = EncodedPacketWithMeta {
                                    data: packet.data,
                                    is_keyframe: packet.is_keyframe,
                                    frame_number,
                                };

                                if let Err(e) = packet_tx.send(meta).await {
                                    error!("Failed to send packet to network: {}", e);
                                    break;
                                }

                                if frame_number % 60 == 0 {
                                    debug!("Encoded {} frames", frame_number);
                                }
                            }
                            Ok(None) => {
                                // Encoder needs more data
                            }
                            Err(e) => {
                                error!("Encoding error: {}", e);
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Encoder loop shutting down");
                        break;
                    }
                }
            }

            // Flush encoder
            if let Ok(packets) = encoder.flush() {
                info!("Flushed {} remaining packets", packets.len());
            }

            info!("Encoder loop stopped");
        });

        Ok(())
    }

    async fn start_network_loop(&mut self) -> Result<()> {
        info!("Initializing network transport...");

        // Create transport
        let network_config = NetworkConfig {
            transport: match self.config.network.transport {
                crate::config::Transport::WebRTC => kd_network::TransportType::WebRTC,
                crate::config::Transport::UDP => kd_network::TransportType::UDP,
            },
            bind_addr: format!("{}:{}",
                               self.config.network.bind_address,
                               self.config.network.port)
                .parse()?,
            max_packet_size: self.config.network.max_packet_size,
            buffer_size: 1024,
            enable_fec: false,
            enable_retransmission: false,
        };

        let mut transport = TransportFactory::create(network_config.transport)
            .map_err(|e| anyhow::anyhow!("Transport init failed: {}", e))?;

        transport.init(network_config).await
            .map_err(|e| anyhow::anyhow!("Transport config failed: {}", e))?;

        info!("✓ Network transport initialized: {:?} on port {}",
              self.config.network.transport,
              self.config.network.port);

        // Spawn network task
        let mut packet_rx = self.packet_rx.take().unwrap();
        let metrics = self.metrics.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            info!("Network loop started");
            let mut sequence = 0u32;

            loop {
                tokio::select! {
                    Some(encoded_packet) = packet_rx.recv() => {
                        // Create network packet
                        let mut packet = Packet::new(
                            PacketType::Video,
                            sequence,
                            encoded_packet.data,
                        );

                        if encoded_packet.is_keyframe {
                            packet = packet.with_flags(kd_network::packet::FLAG_KEYFRAME);
                        }

                        sequence = sequence.wrapping_add(1);

                        // Serialize and send
                        let wire_data = packet.to_bytes();

                        match transport.send(wire_data).await {
                            Ok(_) => {
                                // Update metrics
                                let mut m = metrics.write().await;
                                m.packets_sent += 1;
                                m.bytes_sent += packet.payload.len() as u64;

                                if sequence % 60 == 0 {
                                    debug!("Sent {} packets", sequence);
                                }
                            }
                            Err(e) => {
                                // Don't spam errors for connection issues
                                if sequence % 100 == 0 {
                                    warn!("Network send error: {}", e);
                                }
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Network loop shutting down");
                        break;
                    }
                }
            }

            let _ = transport.disconnect().await;
            info!("Network loop stopped");
        });

        Ok(())
    }

    async fn start_input_loop(&mut self) -> Result<()> {
        info!("Initializing input handler...");

        // Create input handler
        let mut input_handler = InputHandler::new()
            .map_err(|e| anyhow::anyhow!("Input init failed: {}", e))?;

        let input_config = InputConfig {
            enable_keyboard: self.config.input.keyboard_enabled,
            enable_mouse: self.config.input.mouse_enabled,
            enable_gamepad: self.config.input.gamepad_enabled,
            mouse_acceleration: 1.0,
            relative_mouse: false,
        };

        input_handler.init(input_config)
            .map_err(|e| anyhow::anyhow!("Input config failed: {}", e))?;

        info!("✓ Input handler initialized");

        // Spawn input task (receives from network)
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            info!("Input loop started");

            // TODO: Receive input events from network and inject them
            // For now, just wait for shutdown

            let _ = shutdown_rx.recv().await;

            let _ = input_handler.shutdown();
            info!("Input loop stopped");
        });

        Ok(())
    }

    async fn start_metrics_loop(&self) -> Result<()> {
        let metrics = self.metrics.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5));
            let mut last_captured = 0u64;
            let mut last_encoded = 0u64;
            let mut last_sent = 0u64;

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let m = metrics.read().await;

                        let captured_delta = m.frames_captured - last_captured;
                        let encoded_delta = m.frames_encoded - last_encoded;
                        let sent_delta = m.packets_sent - last_sent;

                        let capture_fps = captured_delta as f64 / 5.0;
                        let encode_fps = encoded_delta as f64 / 5.0;
                        let send_fps = sent_delta as f64 / 5.0;

                        let bitrate_kbps = (m.bytes_sent - last_sent as u64) * 8 / 5 / 1000;

                        info!("📊 Metrics: capture={:.1} fps, encode={:.1} fps, send={:.1} fps, bitrate={} kbps",
                              capture_fps, encode_fps, send_fps, bitrate_kbps);

                        last_captured = m.frames_captured;
                        last_encoded = m.frames_encoded;
                        last_sent = m.packets_sent;
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping streaming server...");

        // Send shutdown signal to all tasks
        let _ = self.shutdown_tx.send(());

        // Give tasks time to cleanup
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Print final statistics
        let metrics = self.metrics.read().await;
        info!("Final statistics:");
        info!("  Frames captured: {}", metrics.frames_captured);
        info!("  Frames encoded: {}", metrics.frames_encoded);
        info!("  Packets sent: {}", metrics.packets_sent);
        info!("  Total bytes sent: {} MB", metrics.bytes_sent / 1_000_000);
        info!("  Uptime: {} seconds", metrics.uptime_secs());

        info!("Server stopped");
        Ok(())
    }
}