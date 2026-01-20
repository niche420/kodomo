use crate::config::Config;
use crate::metrics::MetricsCollector;
use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

use kd_capture::{CaptureConfig, CapturedFrame, ScreenCaptureManager};
use kd_encoder::{EncoderConfig, EncoderFactory, RawFrame, VideoEncoder, PixelFormat};
use kd_network::{NetworkConfig, NetworkTransport, TransportFactory};
use bytes::Bytes;

// TUNED: Smaller buffers with backpressure
const FRAME_CHANNEL_SIZE: usize = 3;  // Only buffer 3 frames
const PACKET_CHANNEL_SIZE: usize = 60; // 1 second at 60fps

pub struct StreamingServer {
    config: Config,
    metrics: Arc<RwLock<MetricsCollector>>,
    shutdown_tx: broadcast::Sender<()>,

    // Channels for pipeline
    frame_tx: mpsc::Sender<CapturedFrame>,
    frame_rx: Option<mpsc::Receiver<CapturedFrame>>,

    packet_tx: mpsc::Sender<Bytes>,
    packet_rx: Option<mpsc::Receiver<Bytes>>,
}

// OPTIMIZED: Smart frame handler with aggressive dropping
struct SmartFrameHandler {
    frame_tx: mpsc::Sender<CapturedFrame>,
    frame_count: Arc<AtomicU64>,
    dropped_count: Arc<AtomicU64>,
}

impl SmartFrameHandler {
    fn new(frame_tx: mpsc::Sender<CapturedFrame>) -> Self {
        Self {
            frame_tx,
            frame_count: Arc::new(AtomicU64::new(0)),
            dropped_count: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl kd_capture::CaptureHandler for SmartFrameHandler {
    fn on_frame_arrived(&mut self, frame: CapturedFrame) -> kd_capture::Result<()> {
        let count = self.frame_count.fetch_add(1, Ordering::Relaxed) + 1;

        // CRITICAL: Use try_send for instant backpressure
        match self.frame_tx.try_send(frame) {
            Ok(_) => {
                if count % 300 == 0 {
                    debug!("Captured {} frames", count);
                }
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                // Drop frame immediately if encoder is behind
                let dropped = self.dropped_count.fetch_add(1, Ordering::Relaxed) + 1;

                if dropped % 60 == 0 {
                    warn!("Encoder overloaded - dropped {} frames total", dropped);
                }
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                error!("Frame channel closed");
                return Err(kd_capture::CaptureError::CaptureFailed("Channel closed".into()));
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

        self.start_capture_loop().await?;
        self.start_encoder_loop().await?;
        self.start_network_loop().await?;
        self.start_metrics_loop().await?;

        info!("🚀 Server running on {}:{}",
              self.config.network.bind_address,
              self.config.network.port);

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

        let handler = Arc::new(Mutex::new(SmartFrameHandler::new(
            self.frame_tx.clone(),
        )));

        let capture = Arc::new(Mutex::new(ScreenCaptureManager::new()?));

        // Spawn blocking capture thread
        let capture_config_clone = capture_config.clone();
        let mut capture_start = capture.clone();
        tokio::task::spawn_blocking(move || {
            info!("Capture thread started");
            if let Ok(mut c) = capture_start.lock() {
                let _ = c.start(capture_config_clone, handler);
            }
        });

        // Spawn shutdown listener
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        tokio::spawn(async move {
            let _ = shutdown_rx.recv().await;
            if let Ok(c) = capture.lock() {
                let _ = c.stop();
            }
        });

        info!("✓ Capture configured: {}x{} @ {} FPS",
              self.config.video.width,
              self.config.video.height,
              self.config.video.fps);

        Ok(())
    }

    async fn start_encoder_loop(&mut self) -> Result<()> {
        info!("Initializing encoder...");

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

        let mut encoder = EncoderFactory::create(encoder_config.clone())?;
        encoder.init(encoder_config)?;

        info!("✓ Encoder initialized");

        let mut frame_rx = self.frame_rx.take().unwrap();
        let packet_tx = self.packet_tx.clone();
        let metrics = self.metrics.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        // Spawn BLOCKING encoder thread (FFmpeg is not async)
        tokio::task::spawn_blocking(move || {
            info!("Encoder thread started");
            let rt = tokio::runtime::Handle::current();
            let mut frame_number = 0u64;

            loop {
                // Check shutdown
                if shutdown_rx.try_recv().is_ok() {
                    info!("Encoder shutting down");
                    break;
                }

                // Receive frame (blocking)
                let captured_frame = match frame_rx.blocking_recv() {
                    Some(f) => f,
                    None => break,
                };

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

                // Encode (blocking)
                match encoder.encode(&raw_frame) {
                    Ok(Some(packet)) => {
                        // Update metrics
                        rt.block_on(async {
                            let mut m = metrics.write().await;
                            m.frames_encoded += 1;
                            m.bytes_encoded += packet.data.len() as u64;
                        });

                        // Send to network (blocking)
                        if packet_tx.blocking_send(packet.data).is_err() {
                            warn!("Packet channel closed");
                            break;
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        error!("Encoding error: {}", e);
                    }
                }
            }

            let _ = encoder.flush();
            info!("Encoder thread stopped");
        });

        Ok(())
    }

    async fn start_network_loop(&mut self) -> Result<()> {
        info!("Initializing network...");

        let network_config = NetworkConfig {
            transport: match self.config.network.transport {
                crate::config::Transport::WebRTC => kd_network::TransportType::WebRTC,
                crate::config::Transport::UDP => kd_network::TransportType::UDP,
            },
            bind_addr: format!("{}:{}",
                               self.config.network.bind_address,
                               self.config.network.port).parse()?,
            max_packet_size: self.config.network.max_packet_size,
            buffer_size: 1024,
            enable_fec: false,
            enable_retransmission: false,
        };

        let mut transport = TransportFactory::create(network_config.transport)?;
        transport.init(network_config).await?;

        info!("✓ Network initialized: {:?} on port {}",
              self.config.network.transport,
              self.config.network.port);

        let mut packet_rx = self.packet_rx.take().unwrap();
        let metrics = self.metrics.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            info!("Network loop started");
            let transport = Arc::new(tokio::sync::Mutex::new(transport));

            // Spawn receive task to detect client
            let transport_recv = transport.clone();
            tokio::spawn(async move {
                loop {
                    let mut t = transport_recv.lock().await;
                    if let Ok(data) = t.recv().await {
                        debug!("Received {} bytes from client", data.len());
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            });

            // Send loop
            loop {
                tokio::select! {
                    Some(packet_data) = packet_rx.recv() => {
                        let mut t = transport.lock().await;

                        // SIMPLIFIED: Send as-is, no fragmentation
                        match t.send(packet_data).await {
                            Ok(_) => {
                                let mut m = metrics.write().await;
                                m.packets_sent += 1;
                            }
                            Err(e) => {
                                debug!("Send error: {}", e);
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }

            let mut t = transport.lock().await;
            let _ = t.disconnect().await;
            info!("Network loop stopped");
        });

        Ok(())
    }

    async fn start_metrics_loop(&self) -> Result<()> {
        let metrics = self.metrics.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let m = metrics.read().await;
                        info!("📊 Captured: {}, Encoded: {}, Sent: {}",
                              m.frames_captured, m.frames_encoded, m.packets_sent);
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
        info!("Stopping server...");
        let _ = self.shutdown_tx.send(());
        tokio::time::sleep(Duration::from_millis(500)).await;

        let metrics = self.metrics.read().await;
        info!("Final stats:");
        info!("  Captured: {}", metrics.frames_captured);
        info!("  Encoded: {}", metrics.frames_encoded);
        info!("  Sent: {}", metrics.packets_sent);

        Ok(())
    }
}