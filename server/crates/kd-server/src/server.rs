use crate::config::Config;
use crate::metrics::MetricsCollector;
use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

use kd_capture::{CaptureConfig, CapturedFrame, ScreenCapture, ScreenCaptureManager};
use kd_encoder::{EncoderConfig, EncoderFactory, RawFrame, VideoEncoder, PixelFormat};
use kd_network::{NetworkConfig, NetworkTransport, TransportFactory, Packet, PacketType};
use kd_input::{InputHandler, InputEvent, InputConfig};
use bytes::Bytes;

const FRAME_CHANNEL_SIZE: usize = 240; // 2 seconds at 60fps
const PACKET_CHANNEL_SIZE: usize = 240; // 4 seconds worth

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

// OPTIMIZED: Smart frame handler with better dropping strategy
struct SmartFrameHandler {
    frame_tx: mpsc::Sender<CapturedFrame>,
    frame_count: Arc<AtomicU64>,
    dropped_count: Arc<AtomicU64>,
    last_log: Arc<Mutex<std::time::Instant>>,
}

impl SmartFrameHandler {
    fn new(frame_tx: mpsc::Sender<CapturedFrame>) -> Self {
        Self {
            frame_tx,
            frame_count: Arc::new(AtomicU64::new(0)),
            dropped_count: Arc::new(AtomicU64::new(0)),
            last_log: Arc::new(Mutex::new(std::time::Instant::now())),
        }
    }
}

impl kd_capture::CaptureHandler for SmartFrameHandler {
    fn on_frame_arrived(&mut self, frame: CapturedFrame) -> kd_capture::Result<()> {
        let count = self.frame_count.fetch_add(1, Ordering::Relaxed) + 1;

        // Smart dropping strategy
        match self.frame_tx.try_send(frame) {
            Ok(_) => {
                // Log periodically
                if let Ok(mut last) = self.last_log.try_lock() {
                    if last.elapsed().as_secs() >= 5 {
                        let dropped = self.dropped_count.load(Ordering::Relaxed);
                        if dropped > 0 {
                            info!("Captured {} frames, dropped {} total", count, dropped);
                        } else {
                            debug!("Captured {} frames, no drops", count);
                        }
                        *last = std::time::Instant::now();
                    }
                }
            }
            Err(mpsc::error::TrySendError::Full(frame)) => {
                let dropped = self.dropped_count.fetch_add(1, Ordering::Relaxed) + 1;

                // Only log every 60 drops to avoid spam
                if dropped % 60 == 0 {
                    warn!("Frame buffer full - dropped {} total frames (encoder overloaded)", dropped);
                }

                // Recovery: every 60 dropped frames, force-send to prevent starvation
                if dropped % 60 == 0 {
                    if let Err(e) = self.frame_tx.blocking_send(frame) {
                        error!("Failed to force-send recovery frame: {}", e);
                    }
                }
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                error!("Frame channel closed - stopping capture");
                return Err(kd_capture::CaptureError::CaptureFailed(
                    "Channel closed".into()
                ));
            }
        }

        Ok(())
    }

    fn on_capture_closed(&mut self) {
        info!("Capture closed - total dropped: {}",
              self.dropped_count.load(Ordering::Relaxed));
    }
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

        let capture_config = CaptureConfig {
            mode: self.config.capture.mode.clone(),
            width: self.config.video.width,
            height: self.config.video.height,
            fps: self.config.video.fps,
        };

        info!("✓ Screen capture configured: {}x{} @ {} FPS",
          self.config.video.width,
          self.config.video.height,
          self.config.video.fps);

        // OPTIMIZED: Use smart handler
        let handler = Arc::new(Mutex::new(SmartFrameHandler::new(
            self.frame_tx.clone(),
        )));

        // Create capture manager for stopping
        let capture = Arc::new(Mutex::new(kd_capture::ScreenCaptureManager::new()?));

        // Spawn capture in blocking thread
        let capture_config_clone = capture_config.clone();
        let mut capture_start = capture.clone();
        tokio::task::spawn_blocking(move || {
            info!("Starting capture (blocking)...");

            // This blocks until stopped
            if let Ok(mut c) = capture_start.lock() {
                c.start(capture_config_clone, handler);
            }

            info!("Capture thread exited");
        });

        // Spawn shutdown listener
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        tokio::spawn(async move {
            let _ = shutdown_rx.recv().await;
            info!("Stopping capture...");
            if let Ok(c) = capture.lock() {
                c.stop();
            }
        });

        Ok(())
    }

    async fn start_encoder_loop(&mut self) -> Result<()> {
        info!("Initializing video encoder...");

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

        let mut encoder = EncoderFactory::create(encoder_config.clone())
            .map_err(|e| anyhow::anyhow!("Encoder init failed: {}", e))?;

        encoder.init(encoder_config.clone())
            .map_err(|e| anyhow::anyhow!("Encoder config failed: {}", e))?;

        info!("✓ Video encoder initialized: {:?}, HW accel: {}",
          self.config.video.codec,
          self.config.video.hw_accel);

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

                    match encoder.encode(&raw_frame) {
                        Ok(Some(packet)) => {
                            {
                                let mut m = metrics.write().await;
                                m.frames_encoded += 1;
                                m.bytes_encoded += packet.data.len() as u64;
                            }

                            // CRITICAL: Verify keyframes contain SPS/PPS/IDR
                            if packet.is_keyframe {
                                let has_nal = has_sps_pps_idr(&packet.data);
                                if !has_nal {
                                    error!("❌ Keyframe missing SPS/PPS/IDR! This will break clients!");
                                } else {
                                    info!("✅ Keyframe #{} verified: {} bytes with SPS/PPS/IDR",
                                          frame_number, packet.data.len());
                                }
                            }

                            let meta = EncodedPacketWithMeta {
                                data: packet.data,
                                is_keyframe: packet.is_keyframe,
                                frame_number,
                            };

                            let _ = packet_tx.send(meta).await;
                        }
                        Ok(None) => {}
                        Err(e) => {
                            error!("❌ Encoding error on frame {}: {}", frame_number, e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Encoder loop shutting down");
                    break;
                }
            }
            }

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
            let mut client_connected = false;

            // For UDP server: spawn a receive task to detect client connection
            let transport_clone = Arc::new(tokio::sync::Mutex::new(transport));
            let transport_recv = transport_clone.clone();

            tokio::spawn(async move {
                loop {
                    let mut t = transport_recv.lock().await;
                    match t.recv().await {
                        Ok(data) => {
                            debug!("Received {} bytes from client (likely control/input data)", data.len());
                            // TODO: Handle input packets
                        }
                        Err(e) => {
                            debug!("Receive error: {}", e);
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                    }
                }
            });

            loop {
                tokio::select! {
                    Some(encoded_packet) = packet_rx.recv() => {
                        // Wait a bit for client to connect on first packet
                        if !client_connected {
                            info!("Waiting for client to connect...");
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            client_connected = true;
                        }

                        let mut t = transport_clone.lock().await;
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

                        match t.send(wire_data).await {
                            Ok(_) => {
                                // Update metrics
                                let mut m = metrics.write().await;
                                m.packets_sent += 1;
                                m.bytes_sent += packet.payload.len() as u64;

                                if sequence % 300 == 0 {
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

            let mut t = transport_clone.lock().await;
            let _ = t.disconnect().await;
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
            let mut last_bytes = 0u64;

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let m = metrics.read().await;

                        let captured_delta = m.frames_captured.saturating_sub(last_captured);
                        let encoded_delta = m.frames_encoded.saturating_sub(last_encoded);
                        let sent_delta = m.packets_sent.saturating_sub(last_sent);
                        let bytes_delta = m.bytes_sent.saturating_sub(last_bytes);

                        let capture_fps = captured_delta as f64 / 5.0;
                        let encode_fps = encoded_delta as f64 / 5.0;
                        let send_fps = sent_delta as f64 / 5.0;

                        let bitrate_kbps = bytes_delta * 8 / 5 / 1000;

                        info!("📊 Metrics: capture={:.1} fps, encode={:.1} fps, send={:.1} fps, bitrate={} kbps",
                              capture_fps, encode_fps, send_fps, bitrate_kbps);

                        last_captured = m.frames_captured;
                        last_encoded = m.frames_encoded;
                        last_sent = m.packets_sent;
                        last_bytes = m.bytes_sent;
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

// Helper to verify H.264 Annex-B keyframe structure
fn has_sps_pps_idr(data: &[u8]) -> bool {
    let mut has_sps = false;
    let mut has_pps = false;
    let mut has_idr = false;

    let mut i = 0;
    while i + 4 < data.len() {
        // Find start code (0x00 0x00 0x00 0x01 or 0x00 0x00 0x01)
        let start_code_len = if data[i] == 0x00 && data[i+1] == 0x00 {
            if data[i+2] == 0x00 && i+3 < data.len() && data[i+3] == 0x01 {
                4
            } else if data[i+2] == 0x01 {
                3
            } else {
                i += 1;
                continue;
            }
        } else {
            i += 1;
            continue;
        };

        i += start_code_len;
        if i >= data.len() {
            break;
        }

        let nal_type = data[i] & 0x1F;

        match nal_type {
            7 => has_sps = true,  // SPS
            8 => has_pps = true,  // PPS
            5 => has_idr = true,  // IDR slice
            _ => {}
        }

        i += 1;
    }

    has_sps && has_pps && has_idr
}
