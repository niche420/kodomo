use super::*;
use tracing::{debug, info, warn, error};

#[cfg(feature = "ffmpeg")]
use ffmpeg_next as ffmpeg;

pub struct FfmpegEncoder {
    config: EncoderConfig,
    encoder: Option<ffmpeg_next::codec::encoder::video::Encoder>,
    frame_count: u64,
    initialized: bool,

    // PRE-ALLOCATED BUFFERS (never resize in hot path)
    yuv_buffer: Vec<u8>,
    y_plane: Vec<u8>,
    u_plane: Vec<u8>,
    v_plane: Vec<u8>,
}

unsafe impl Send for FfmpegEncoder {}
unsafe impl Sync for FfmpegEncoder {}

impl FfmpegEncoder {
    pub fn new(config: EncoderConfig) -> Result<Self> {
        info!("Initializing FFmpeg encoder");

        // Pre-allocate all buffers based on config
        let y_size = (config.width * config.height) as usize;
        let uv_size = y_size / 4;

        Ok(Self {
            config,
            encoder: None,
            frame_count: 0,
            initialized: false,
            yuv_buffer: vec![0u8; y_size + uv_size * 2],
            y_plane: vec![0u8; y_size],
            u_plane: vec![0u8; uv_size],
            v_plane: vec![0u8; uv_size],
        })
    }

    pub fn is_available() -> bool {
        #[cfg(feature = "ffmpeg")]
        {
            ffmpeg::init().is_ok()
        }
        #[cfg(not(feature = "ffmpeg"))]
        false
    }

    pub fn is_nvenc_available() -> bool {
        #[cfg(feature = "ffmpeg")]
        {
            if ffmpeg::init().is_err() {
                return false;
            }
            ffmpeg::encoder::find_by_name("h264_nvenc").is_some()
        }
        #[cfg(not(feature = "ffmpeg"))]
        false
    }

    // OPTIMIZED: In-place conversion using pre-allocated buffers
    fn bgra_to_yuv420p_fast(&mut self, bgra: &[u8], width: u32, height: u32) {
        let width_usize = width as usize;
        let height_usize = height as usize;

        // Y plane (luminance) - full resolution
        for y in 0..height_usize {
            for x in 0..width_usize {
                let idx = (y * width_usize + x) * 4;
                let b = bgra[idx] as f32;
                let g = bgra[idx + 1] as f32;
                let r = bgra[idx + 2] as f32;

                // BT.601 conversion
                let y_val = (0.257 * r + 0.504 * g + 0.098 * b + 16.0) as u8;
                self.y_plane[y * width_usize + x] = y_val;
            }
        }

        // U and V planes (chrominance) - subsampled 2x2
        let half_width = width_usize / 2;
        let half_height = height_usize / 2;

        for y in 0..half_height {
            for x in 0..half_width {
                // Sample 2x2 block
                let src_y = y * 2;
                let src_x = x * 2;

                // Top-left pixel
                let idx = (src_y * width_usize + src_x) * 4;
                let b = bgra[idx] as f32;
                let g = bgra[idx + 1] as f32;
                let r = bgra[idx + 2] as f32;

                // BT.601 conversion
                let u_val = (-0.148 * r - 0.291 * g + 0.439 * b + 128.0) as u8;
                let v_val = (0.439 * r - 0.368 * g - 0.071 * b + 128.0) as u8;

                let dst_idx = y * half_width + x;
                self.u_plane[dst_idx] = u_val;
                self.v_plane[dst_idx] = v_val;
            }
        }
    }
}

impl VideoEncoder for FfmpegEncoder {
    fn init(&mut self, config: EncoderConfig) -> Result<()> {
        #[cfg(feature = "ffmpeg")]
        {
            info!("Initializing FFmpeg encoder: {}x{} @ {}fps, {} kbps",
                  config.width, config.height, config.fps, config.bitrate_kbps);

            self.config = config;

            let encoder_name = if self.config.use_hardware && Self::is_nvenc_available() {
                info!("✅ Using NVENC hardware acceleration");
                "h264_nvenc"
            } else {
                warn!("⚠️ Using SOFTWARE encoding (will be slow!)");
                "libx264"
            };

            ffmpeg::init()
                .map_err(|e| EncoderError::InitFailed(format!("FFmpeg init: {}", e)))?;

            let codec = ffmpeg::encoder::find_by_name(encoder_name)
                .ok_or_else(|| EncoderError::InitFailed(format!("Codec '{}' not found", encoder_name)))?;

            let mut context = ffmpeg::codec::context::Context::new_with_codec(codec)
                .encoder()
                .video()
                .map_err(|e| EncoderError::InitFailed(format!("Context: {}", e)))?;

            context.set_width(self.config.width);
            context.set_height(self.config.height);
            context.set_format(ffmpeg::format::Pixel::YUV420P);
            context.set_time_base(ffmpeg::Rational::new(1, self.config.fps as i32));
            context.set_frame_rate(Some(ffmpeg::Rational::new(self.config.fps as i32, 1)));
            context.set_bit_rate(self.config.bitrate_kbps as usize * 1000);
            context.set_max_bit_rate(self.config.bitrate_kbps as usize * 1000);
            context.set_gop(self.config.keyframe_interval);

            let mut opts = ffmpeg::Dictionary::new();

            if encoder_name.contains("nvenc") {
                opts.set("preset", "p4"); // Balanced
                opts.set("tune", "ll"); // Low latency
                opts.set("rc", "cbr"); // Constant bitrate
                opts.set("delay", "0");
                opts.set("zerolatency", "1");
                opts.set("forced-idr", "1"); // Force IDR frames
                opts.set("repeat_headers", "1"); // Inline SPS/PPS
            } else {
                opts.set("preset", "ultrafast");
                opts.set("tune", "zerolatency");
                opts.set("x264-params", "repeat-headers=1:annexb=1");
            }

            let opened_encoder = context
                .open_with(opts)
                .map_err(|e| EncoderError::InitFailed(format!("Open encoder: {}", e)))?;

            self.encoder = Some(opened_encoder);
            self.initialized = true;

            info!("✅ Encoder initialized successfully");
            Ok(())
        }

        #[cfg(not(feature = "ffmpeg"))]
        Err(EncoderError::InitFailed("FFmpeg feature not enabled".into()))
    }

    fn encode(&mut self, frame: &RawFrame) -> Result<Option<EncodedPacket>> {
        #[cfg(feature = "ffmpeg")]
        {
            if !self.initialized {
                return Err(EncoderError::InitFailed("Not initialized".into()));
            }

            // Convert BGRA to YUV420 using pre-allocated buffers
            if frame.format == PixelFormat::BGRA {
                self.bgra_to_yuv420p_fast(&frame.data, frame.width, frame.height);
            }

            // Create FFmpeg frame
            let mut yuv_frame = ffmpeg::frame::Video::new(
                ffmpeg::format::Pixel::YUV420P,
                self.config.width,
                self.config.height,
            );

            // Copy planes (no allocation, direct copy)
            yuv_frame.data_mut(0).copy_from_slice(&self.y_plane);
            yuv_frame.data_mut(1).copy_from_slice(&self.u_plane);
            yuv_frame.data_mut(2).copy_from_slice(&self.v_plane);

            yuv_frame.set_pts(Some(self.frame_count as i64));

            let encoder = self.encoder.as_mut()
                .ok_or_else(|| EncoderError::EncodingFailed("Encoder not available".into()))?;

            // Send frame
            encoder.send_frame(&yuv_frame)
                .map_err(|e| EncoderError::EncodingFailed(format!("Send frame: {}", e)))?;

            // Receive packet
            let mut encoded_packet = ffmpeg::codec::packet::Packet::empty();

            match encoder.receive_packet(&mut encoded_packet) {
                Ok(_) => {
                    let is_keyframe = encoded_packet.is_key();
                    let data = encoded_packet.data().unwrap_or(&[]).to_vec();

                    self.frame_count += 1;

                    Ok(Some(EncodedPacket {
                        data: Bytes::from(data),
                        pts: frame.pts,
                        dts: frame.pts,
                        is_keyframe,
                        timestamp: frame.timestamp,
                        codec: self.config.codec,
                    }))
                }
                Err(ffmpeg::Error::Other { errno: ffmpeg::error::EAGAIN }) => {
                    self.frame_count += 1;
                    Ok(None)
                }
                Err(e) => {
                    Err(EncoderError::EncodingFailed(format!("Receive packet: {}", e)))
                }
            }
        }

        #[cfg(not(feature = "ffmpeg"))]
        Err(EncoderError::EncodingFailed("FFmpeg not enabled".into()))
    }

    fn flush(&mut self) -> Result<Vec<EncodedPacket>> {
        #[cfg(feature = "ffmpeg")]
        {
            if let Some(encoder) = &mut self.encoder {
                let mut packets = Vec::new();

                if let Err(e) = encoder.send_eof() {
                    warn!("Failed to send EOF: {}", e);
                    return Ok(packets);
                }

                loop {
                    let mut packet = ffmpeg::codec::packet::Packet::empty();
                    match encoder.receive_packet(&mut packet) {
                        Ok(_) => {
                            if let Some(data) = packet.data() {
                                packets.push(EncodedPacket {
                                    data: Bytes::from(data.to_vec()),
                                    pts: self.frame_count,
                                    dts: self.frame_count,
                                    is_keyframe: packet.is_key(),
                                    timestamp: std::time::Instant::now(),
                                    codec: self.config.codec,
                                });
                                self.frame_count += 1;
                            }
                        }
                        Err(_) => break,
                    }
                }

                return Ok(packets);
            }
        }

        Ok(vec![])
    }

    fn set_bitrate(&mut self, bitrate_kbps: u32) -> Result<()> {
        self.config.bitrate_kbps = bitrate_kbps;

        #[cfg(feature = "ffmpeg")]
        {
            if let Some(encoder) = &mut self.encoder {
                encoder.set_bit_rate(bitrate_kbps as usize * 1000);
                encoder.set_max_bit_rate(bitrate_kbps as usize * 1000);
            }
        }

        Ok(())
    }

    fn get_config(&self) -> &EncoderConfig {
        &self.config
    }
}

impl Drop for FfmpegEncoder {
    fn drop(&mut self) {
        if self.initialized {
            let _ = self.flush();
            self.encoder = None;
        }
    }
}