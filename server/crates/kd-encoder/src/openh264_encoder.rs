use super::*;
use tracing::{debug, info};

#[cfg(feature = "openh264")]
use openh264::encoder::Encoder as H264Encoder;
#[cfg(feature = "openh264")]
use openh264::formats::YUVBuffer;
use rayon::prelude::*;

pub struct OpenH264Encoder {
    config: EncoderConfig,
    #[cfg(feature = "openh264")]
    encoder: Option<H264Encoder>,
    initialized: bool,
    frame_count: u64,
    #[cfg(feature = "openh264")]
    yuv_buffer: Vec<u8>, // pre-allocated buffer for conversion
}

impl OpenH264Encoder {
    pub fn new(config: EncoderConfig) -> Result<Self> {
        let yuv_size = ((config.width * config.height) + ((config.width * config.height) / 4) * 2) as usize;
        Ok(Self {
            config,
            #[cfg(feature = "openh264")]
            encoder: None,
            initialized: false,
            frame_count: 0,
            #[cfg(feature = "openh264")]
            yuv_buffer: vec![0u8; yuv_size],
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
            info!("Initializing OpenH264 encoder: {}x{} @ {} fps, {} kbps", config.width, config.height, config.fps, config.bitrate_kbps);

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

            // Convert frame data to I420 in parallel if needed
            match frame.format {
                PixelFormat::I420 => {
                    self.yuv_buffer[..frame.data.len()].copy_from_slice(&frame.data);
                }
                PixelFormat::BGRA | PixelFormat::RGBA => {
                    Self::convert_to_i420_parallel(&frame.data, frame.width, frame.height, frame.format, &mut self.yuv_buffer);
                }
                _ => return Err(EncoderError::InvalidConfig("Unsupported pixel format".into())),
            };

            let yuv_source = YUVBuffer::from_vec(self.yuv_buffer.clone(), frame.width as usize, frame.height as usize);

            let encoder = self.encoder.as_mut()
                .ok_or(EncoderError::EncodingFailed("Encoder not available".into()))?;

            let bitstream = encoder.encode(&yuv_source)
                .map_err(|e| EncoderError::EncodingFailed(format!("Encoding failed: {:?}", e)))?;

            let mut encoded_data = Vec::new();
            for layer_idx in 0..bitstream.num_layers() {
                if let Some(layer) = bitstream.layer(layer_idx) {
                    for nal_idx in 0..layer.nal_count() {
                        if let Some(nal_unit) = layer.nal_unit(nal_idx) {
                            encoded_data.extend_from_slice(nal_unit);
                        }
                    }
                }
            }

            if !encoded_data.is_empty() {
                let is_keyframe = self.frame_count % self.config.keyframe_interval as u64 == 0;

                debug!("Encoded frame {}: {} bytes, keyframe: {}", self.frame_count, encoded_data.len(), is_keyframe);

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
        Ok(())
    }

    fn get_config(&self) -> &EncoderConfig {
        &self.config
    }
}

impl OpenH264Encoder {
    fn convert_to_i420_parallel(data: &[u8], width: u32, height: u32, format: PixelFormat, yuv: &mut [u8]) {
        let y_size = (width * height) as usize;
        let uv_size = y_size / 4;
        let (r_off, g_off, b_off) = match format {
            PixelFormat::RGBA => (0, 1, 2),
            PixelFormat::BGRA => (2, 1, 0),
            _ => (0, 1, 2),
        };

        // Y plane
        yuv[..y_size].par_chunks_mut(width as usize).enumerate().for_each(|(row_idx, row)| {
            let y = row_idx as u32;
            for x in 0..width as usize {
                let idx = ((y * width + x as u32) * 4) as usize;
                let r = data[idx + r_off] as f32;
                let g = data[idx + g_off] as f32;
                let b = data[idx + b_off] as f32;
                row[x] = (0.257*r + 0.504*g + 0.098*b + 16.0) as u8;
            }
        });

        let u_offset = y_size;
        let v_offset = y_size + uv_size;

        let width_uv = (width / 2) as usize;
        let height_uv = (height / 2) as usize;

        // Split U/V planes into parallel chunks
        yuv[u_offset..u_offset+uv_size].par_chunks_mut(width_uv).enumerate().for_each(|(row_idx, row)| {
            let y = row_idx * 2;
            for x in 0..width_uv {
                let idx = ((y as u32 * width + (x as u32) * 2) * 4) as usize;
                let r = data[idx + r_off] as f32;
                let g = data[idx + g_off] as f32;
                let b = data[idx + b_off] as f32;
                row[x] = (-0.148*r - 0.291*g + 0.439*b + 128.0) as u8;
            }
        });

        yuv[v_offset..v_offset+uv_size].par_chunks_mut(width_uv).enumerate().for_each(|(row_idx, row)| {
            let y = row_idx * 2;
            for x in 0..width_uv {
                let idx = ((y as u32 * width + (x as u32) * 2) * 4) as usize;
                let r = data[idx + r_off] as f32;
                let g = data[idx + g_off] as f32;
                let b = data[idx + b_off] as f32;
                row[x] = (0.439*r - 0.368*g - 0.071*b + 128.0) as u8;
            }
        });
    }
}
