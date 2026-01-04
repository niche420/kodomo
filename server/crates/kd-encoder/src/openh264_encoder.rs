use super::*;
use tracing::{debug, info};

#[cfg(feature = "openh264")]
use openh264::encoder::Encoder as H264Encoder;
#[cfg(feature = "openh264")]
use openh264::formats::YUVBuffer;

pub struct OpenH264Encoder {
    config: EncoderConfig,
    #[cfg(feature = "openh264")]
    encoder: Option<H264Encoder>,
    initialized: bool,
    frame_count: u64,
}

impl OpenH264Encoder {
    pub fn new(config: EncoderConfig) -> Result<Self> {
        Ok(Self {
            config,
            #[cfg(feature = "openh264")]
            encoder: None,
            initialized: false,
            frame_count: 0,
        })
    }

    pub fn is_available() -> bool {
        cfg!(feature = "openh264")
    }
}

impl VideoEncoder for OpenH264Encoder {
    fn init(&mut self, config: EncoderConfig) -> Result<()> {
        #[cfg(feature = "openh264")]
        {
            info!("Initializing OpenH264 encoder: {}x{} @ {} fps, {} kbps",
                  config.width, config.height, config.fps, config.bitrate_kbps);

            let encoder = H264Encoder::new()
                .map_err(|e| EncoderError::InitFailed(format!("OpenH264 init failed: {:?}", e)))?;

            self.encoder = Some(encoder);
            self.config = config;
            self.initialized = true;

            info!("✓ OpenH264 encoder initialized successfully");
            Ok(())
        }

        #[cfg(not(feature = "openh264"))]
        {
            Err(EncoderError::InitFailed("OpenH264 feature not enabled".into()))
        }
    }

    fn encode(&mut self, frame: &RawFrame) -> Result<Option<EncodedPacket>> {
        #[cfg(feature = "openh264")]
        {
            if !self.initialized {
                return Err(EncoderError::InitFailed("Encoder not initialized".into()));
            }

            // Convert frame data to YUV if needed
            let yuv = match frame.format {
                PixelFormat::I420 => {
                    // Already in correct format
                    frame.data.clone()
                }
                PixelFormat::BGRA | PixelFormat::RGBA => {
                    // Convert to I420
                    Self::convert_to_i420_static(&frame.data, frame.width, frame.height, frame.format)
                }
                _ => return Err(EncoderError::InvalidConfig("Unsupported pixel format".into())),
            };

            // Create YUV buffer
            let yuv_source = YUVBuffer::from_vec(
                yuv,
                frame.width as usize,
                frame.height as usize,
            );

            let encoder = self.encoder.as_mut()
                .ok_or(EncoderError::EncodingFailed("Encoder not available".into()))?;

            // Encode frame
            let bitstream = encoder.encode(&yuv_source)
                .map_err(|e| EncoderError::EncodingFailed(format!("Encoding failed: {:?}", e)))?;

            // Extract NAL units from bitstream
            let mut encoded_data = Vec::new();

            // Iterate through layers (usually just one)
            for layer_idx in 0..bitstream.num_layers() {
                if let Some(layer) = bitstream.layer(layer_idx) {
                    // Get each NAL unit from the layer
                    for nal_idx in 0..layer.nal_count() {
                        if let Some(nal_unit) = layer.nal_unit(nal_idx) {
                            encoded_data.extend_from_slice(nal_unit);
                        }
                    }
                }
            }

            if !encoded_data.is_empty() {
                let is_keyframe = self.frame_count % self.config.keyframe_interval as u64 == 0;

                debug!("Encoded frame {}: {} bytes, keyframe: {}",
                       self.frame_count, encoded_data.len(), is_keyframe);

                self.frame_count += 1;

                return Ok(Some(EncodedPacket {
                    data: Bytes::from(encoded_data),
                    pts: frame.pts,
                    dts: frame.pts,
                    is_keyframe,
                    timestamp: frame.timestamp,
                    codec: self.config.codec,
                }));
            }

            Ok(None)
        }

        #[cfg(not(feature = "openh264"))]
        {
            Err(EncoderError::EncodingFailed("OpenH264 feature not enabled".into()))
        }
    }

    fn flush(&mut self) -> Result<Vec<EncodedPacket>> {
        Ok(vec![])
    }

    fn set_bitrate(&mut self, bitrate_kbps: u32) -> Result<()> {
        info!("Updating bitrate to {} kbps", bitrate_kbps);
        self.config.bitrate_kbps = bitrate_kbps;
        // TODO: Update encoder bitrate dynamically
        Ok(())
    }

    fn get_config(&self) -> &EncoderConfig {
        &self.config
    }
}

impl OpenH264Encoder {
    fn convert_to_i420_static(data: &[u8], width: u32, height: u32, format: PixelFormat) -> Vec<u8> {
        let y_size = (width * height) as usize;
        let uv_size = y_size / 4;
        let mut yuv = vec![0u8; y_size + uv_size * 2];

        let (r_off, g_off, b_off) = match format {
            PixelFormat::RGBA => (0, 1, 2),
            PixelFormat::BGRA => (2, 1, 0),
            _ => (0, 1, 2),
        };

        // Y plane
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                let r = data[idx + r_off] as f32;
                let g = data[idx + g_off] as f32;
                let b = data[idx + b_off] as f32;

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
                let r = data[idx + r_off] as f32;
                let g = data[idx + g_off] as f32;
                let b = data[idx + b_off] as f32;

                let u_val = (-0.148 * r - 0.291 * g + 0.439 * b + 128.0) as u8;
                let v_val = (0.439 * r - 0.368 * g - 0.071 * b + 128.0) as u8;

                let uv_idx = ((y / 2) * (width / 2) + (x / 2)) as usize;
                yuv[u_offset + uv_idx] = u_val;
                yuv[v_offset + uv_idx] = v_val;
            }
        }

        yuv
    }
}
