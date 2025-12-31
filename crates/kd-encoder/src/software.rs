use super::*;
use tracing::{debug, info};

/// Software encoder using x264 (simple implementation)
/// In production, you'd use FFmpeg or a proper x264 binding
pub struct SoftwareEncoder {
    config: EncoderConfig,
    frame_count: u64,
    initialized: bool,
}

impl SoftwareEncoder {
    pub fn new(config: EncoderConfig) -> Result<Self> {
        Ok(Self {
            config,
            frame_count: 0,
            initialized: false,
        })
    }

    fn convert_bgra_to_yuv420(&self, bgra: &[u8], width: u32, height: u32) -> Vec<u8> {
        let y_size = (width * height) as usize;
        let uv_size = y_size / 4;
        let mut yuv = vec![0u8; y_size + uv_size * 2];

        // Simple BGRA to YUV420 conversion
        // Y plane
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                let b = bgra[idx] as f32;
                let g = bgra[idx + 1] as f32;
                let r = bgra[idx + 2] as f32;

                let y_val = (0.257 * r + 0.504 * g + 0.098 * b + 16.0) as u8;
                yuv[(y * width + x) as usize] = y_val;
            }
        }

        // U and V planes (subsampled)
        let u_offset = y_size;
        let v_offset = y_size + uv_size;

        for y in (0..height).step_by(2) {
            for x in (0..width).step_by(2) {
                let idx = ((y * width + x) * 4) as usize;
                let b = bgra[idx] as f32;
                let g = bgra[idx + 1] as f32;
                let r = bgra[idx + 2] as f32;

                let u_val = (-0.148 * r - 0.291 * g + 0.439 * b + 128.0) as u8;
                let v_val = (0.439 * r - 0.368 * g - 0.071 * b + 128.0) as u8;

                let uv_idx = ((y / 2) * (width / 2) + (x / 2)) as usize;
                yuv[u_offset + uv_idx] = u_val;
                yuv[v_offset + uv_idx] = v_val;
            }
        }

        yuv
    }

    fn encode_yuv(&mut self, yuv: &[u8], pts: u64, is_keyframe: bool) -> Result<EncodedPacket> {
        // In a real implementation, this would use actual x264 encoding
        // For now, we'll simulate encoding with compression

        // Simulate H.264 NAL units
        let mut encoded = Vec::new();

        if is_keyframe {
            // SPS (Sequence Parameter Set)
            encoded.extend_from_slice(&[0, 0, 0, 1, 0x67]); // NAL unit header for SPS
            encoded.extend_from_slice(&[0x42, 0x00, 0x1f]); // Profile/level

            // PPS (Picture Parameter Set)
            encoded.extend_from_slice(&[0, 0, 0, 1, 0x68]); // NAL unit header for PPS

            // IDR frame
            encoded.extend_from_slice(&[0, 0, 0, 1, 0x65]); // NAL unit header for IDR
        } else {
            // P-frame
            encoded.extend_from_slice(&[0, 0, 0, 1, 0x41]); // NAL unit header for P-frame
        }

        // Simulate compression (in reality, x264 does this)
        // Just take a subset to simulate compression ratio
        let compression_ratio = if is_keyframe { 0.1 } else { 0.05 };
        let compressed_size = (yuv.len() as f32 * compression_ratio) as usize;
        encoded.extend_from_slice(&yuv[..compressed_size.min(yuv.len())]);

        debug!("Encoded frame {}: {} bytes, keyframe: {}",
               self.frame_count, encoded.len(), is_keyframe);

        Ok(EncodedPacket {
            data: Bytes::from(encoded),
            pts,
            dts: pts,
            is_keyframe,
            timestamp: Instant::now(),
            codec: self.config.codec,
        })
    }
}

impl VideoEncoder for SoftwareEncoder {
    fn init(&mut self, config: EncoderConfig) -> Result<()> {
        info!("Initializing software encoder: {}x{} @ {} fps, {} kbps",
              config.width, config.height, config.fps, config.bitrate_kbps);

        self.config = config;
        self.initialized = true;
        Ok(())
    }

    fn encode(&mut self, frame: &RawFrame) -> Result<Option<EncodedPacket>> {
        if !self.initialized {
            return Err(EncoderError::InitFailed("Encoder not initialized".into()));
        }

        // Convert to YUV420 if needed
        let yuv = match frame.format {
            PixelFormat::BGRA => {
                self.convert_bgra_to_yuv420(&frame.data, frame.width, frame.height)
            }
            PixelFormat::I420 => frame.data.clone(),
            _ => return Err(EncoderError::InvalidConfig("Unsupported pixel format".into())),
        };

        // Determine if this should be a keyframe
        let is_keyframe = self.frame_count % self.config.keyframe_interval as u64 == 0;

        let packet = self.encode_yuv(&yuv, frame.pts, is_keyframe)?;

        self.frame_count += 1;
        Ok(Some(packet))
    }

    fn flush(&mut self) -> Result<Vec<EncodedPacket>> {
        // Return any buffered frames
        Ok(vec![])
    }

    fn set_bitrate(&mut self, bitrate_kbps: u32) -> Result<()> {
        info!("Updating bitrate to {} kbps", bitrate_kbps);
        self.config.bitrate_kbps = bitrate_kbps;
        Ok(())
    }

    fn get_config(&self) -> &EncoderConfig {
        &self.config
    }
}