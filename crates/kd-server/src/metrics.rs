use std::time::Instant;

#[derive(Debug, Default)]
pub struct MetricsCollector {
    pub frames_captured: u64,
    pub frames_encoded: u64,
    pub packets_sent: u64,
    pub frames_dropped: u64,
    pub bytes_sent: u64,
    pub bytes_encoded: u64,
    pub start_time: Option<Instant>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            start_time: Some(Instant::now()),
            ..Default::default()
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        self.start_time
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0)
    }

    pub fn average_fps(&self) -> f64 {
        let uptime = self.uptime_secs();
        if uptime > 0 {
            self.frames_captured as f64 / uptime as f64
        } else {
            0.0
        }
    }

    pub fn average_bitrate_kbps(&self) -> f64 {
        let uptime = self.uptime_secs();
        if uptime > 0 {
            (self.bytes_sent * 8) as f64 / uptime as f64 / 1000.0
        } else {
            0.0
        }
    }
}