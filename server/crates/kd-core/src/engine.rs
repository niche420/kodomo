use crate::{config::StreamConfig, error::Result, types::*, StreamError};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{debug, error, info, warn};
use kd_encoder::VideoCodec;

const FRAME_CHANNEL_SIZE: usize = 32;
const ENCODED_CHANNEL_SIZE: usize = 64;

pub struct StreamingEngine {
    config: Arc<RwLock<StreamConfig>>,
    running: Arc<RwLock<bool>>,

    // Channels for pipeline
    frame_tx: mpsc::Sender<Frame>,
    frame_rx: Option<mpsc::Receiver<Frame>>,

    encoded_tx: mpsc::Sender<EncodedPacket>,
    encoded_rx: Option<mpsc::Receiver<EncodedPacket>>,

    // Shutdown signal
    shutdown_tx: broadcast::Sender<()>,

    // Statistics
    stats: Arc<RwLock<StreamStats>>,
}

#[derive(Debug, Default, Clone)]
pub struct StreamStats {
    pub frames_captured: u64,
    pub frames_encoded: u64,
    pub frames_sent: u64,
    pub frames_dropped: u64,
    pub bytes_sent: u64,
    pub average_encode_time_ms: f64,
    pub average_network_latency_ms: f64,
}

impl StreamingEngine {
    pub fn new(config: StreamConfig) -> Self {
        let (frame_tx, frame_rx) = mpsc::channel(FRAME_CHANNEL_SIZE);
        let (encoded_tx, encoded_rx) = mpsc::channel(ENCODED_CHANNEL_SIZE);
        let (shutdown_tx, _) = broadcast::channel(1);

        info!("Created streaming engine with config: {:?}", config);

        Self {
            config: Arc::new(RwLock::new(config)),
            running: Arc::new(RwLock::new(false)),
            frame_tx,
            frame_rx: Some(frame_rx),
            encoded_tx,
            encoded_rx: Some(encoded_rx),
            shutdown_tx,
            stats: Arc::new(RwLock::new(StreamStats::default())),
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        {
            let mut running = self.running.write().await;
            if *running {
                return Err(StreamError::AlreadyRunning);
            }

            *running = true;
        }

        info!("Starting streaming engine");

        // Start subsystems
        self.start_capture_loop().await?;
        self.start_encoding_loop().await?;
        self.start_network_loop().await?;

        info!("Streaming engine started successfully");
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        let mut running = self.running.write().await;

        if !*running {
            return Err(StreamError::NotRunning);
        }

        info!("Stopping streaming engine");

        // Send shutdown signal to all subsystems
        let _ = self.shutdown_tx.send(());

        *running = false;
        info!("Streaming engine stopped");
        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    pub async fn update_config(&mut self, new_config: StreamConfig) -> Result<()> {
        let mut config = self.config.write().await;
        info!("Updating configuration");
        *config = new_config;
        Ok(())
    }

    pub async fn get_stats(&self) -> StreamStats {
        self.stats.read().await.clone()
    }

    // Get channels for subsystems to use
    pub fn get_frame_sender(&self) -> mpsc::Sender<Frame> {
        self.frame_tx.clone()
    }

    pub fn get_encoded_sender(&self) -> mpsc::Sender<EncodedPacket> {
        self.encoded_tx.clone()
    }

    pub fn subscribe_shutdown(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    async fn start_capture_loop(&mut self) -> Result<()> {
        debug!("Starting capture loop");
        // Will be implemented by capture crate
        Ok(())
    }

    async fn start_encoding_loop(&mut self) -> Result<()> {
        debug!("Starting encoding loop");

        let mut frame_rx = self.frame_rx.take()
            .ok_or(StreamError::Config("Frame receiver already taken".into()))?;
        let encoded_tx = self.encoded_tx.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let stats = self.stats.clone();

        tokio::spawn(async move {
            info!("Encoding loop started");

            loop {
                tokio::select! {
                    Some(frame) = frame_rx.recv() => {
                        // TODO: Actually encode the frame
                        // For now, just pass through as a mock encoded packet
                        let packet = EncodedPacket {
                            data: bytes::Bytes::from(frame.data),
                            timestamp: frame.timestamp,
                            frame_number: frame.frame_number,
                            is_keyframe: frame.frame_number % 60 == 0,
                            codec: VideoCodec::H264,
                        };

                        if encoded_tx.send(packet).await.is_err() {
                            error!("Encoded channel closed");
                            break;
                        }

                        let mut s = stats.write().await;
                        s.frames_encoded += 1;
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Encoding loop shutting down");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    async fn start_network_loop(&mut self) -> Result<()> {
        debug!("Starting network loop");

        let mut encoded_rx = self.encoded_rx.take()
            .ok_or(StreamError::Config("Encoded receiver already taken".into()))?;
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let stats = self.stats.clone();

        tokio::spawn(async move {
            info!("Network loop started");

            loop {
                tokio::select! {
                    Some(packet) = encoded_rx.recv() => {
                        // TODO: Actually send over network
                        // For now, just count it
                        let mut s = stats.write().await;
                        s.frames_sent += 1;
                        s.bytes_sent += packet.size() as u64;

                        debug!("Sent packet: {} bytes, keyframe: {}",
                               packet.size(), packet.is_keyframe);
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Network loop shutting down");
                        break;
                    }
                }
            }
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_engine_lifecycle() {
        let config = StreamConfig::default();
        let mut engine = StreamingEngine::new(config);

        assert!(!engine.is_running().await);

        engine.start().await.unwrap();
        assert!(engine.is_running().await);

        engine.stop().await.unwrap();
        assert!(!engine.is_running().await);
    }

    #[tokio::test]
    async fn test_double_start_fails() {
        let config = StreamConfig::default();
        let mut engine = StreamingEngine::new(config);

        engine.start().await.unwrap();
        let result = engine.start().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_config_update() {
        let config = StreamConfig::default();
        let mut engine = StreamingEngine::new(config);

        let mut new_config = StreamConfig::default();
        new_config.video.bitrate_kbps = 5000;

        engine.update_config(new_config).await.unwrap();

        let config = engine.config.read().await;
        assert_eq!(config.video.bitrate_kbps, 5000);
    }
}