use super::*;
use tracing::{debug, info, warn, error};

#[cfg(feature = "ffmpeg")]
use ffmpeg_next as ffmpeg;

pub struct FfmpegEncoder {
    config: EncoderConfig,
    encoder: Option<ffmpeg_next::codec::encoder::video::Encoder>,
    frame_count: u64,
    initialized: bool,
    // FIXED: Pre-allocated buffer for YUV conversion
    yuv_buffer: Vec<u8>,
}

// FFmpeg types are not Send by default, but we're using them single-threaded
// within the encoder task, so this is safe
unsafe impl Send for FfmpegEncoder {}
unsafe impl Sync for FfmpegEncoder {}

impl FfmpegEncoder {
    pub fn new(config: EncoderConfig) -> Result<Self> {
        info!("Initializing FFmpeg encoder");

        // FIXED: Pre-allocate YUV buffer to avoid allocations in hot path
        let yuv_size = ((config.width * config.height * 3) / 2) as usize;

        Ok(Self {
            config,
            encoder: None,
            frame_count: 0,
            initialized: false,
            yuv_buffer: vec![0u8; yuv_size],
        })
    }

    pub fn is_available() -> bool {
        #[cfg(feature = "ffmpeg")]
        {
            // Check if FFmpeg is available
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

            // Try to find h264_nvenc encoder
            ffmpeg::encoder::find_by_name("h264_nvenc").is_some()
        }
        #[cfg(not(feature = "ffmpeg"))]
        false
    }

    fn preset_to_ffmpeg(&self, encoder_name: &str) -> &str {
        if encoder_name.contains("nvenc") {
            // NVENC presets (p1-p7, higher = slower/better quality)
            match self.config.preset {
                EncoderPreset::UltraFast => "p1",
                EncoderPreset::SuperFast => "p2",
                EncoderPreset::VeryFast => "p3",
                EncoderPreset::Faster => "p4",
                EncoderPreset::Fast => "p4",
                EncoderPreset::Medium => "p5",
                EncoderPreset::Slow => "p6",
                EncoderPreset::Slower => "p7",
                EncoderPreset::VerySlow => "p7",
            }
        } else {
            // x264/x265 presets
            match self.config.preset {
                EncoderPreset::UltraFast => "ultrafast",
                EncoderPreset::SuperFast => "superfast",
                EncoderPreset::VeryFast => "veryfast",
                EncoderPreset::Faster => "faster",
                EncoderPreset::Fast => "fast",
                EncoderPreset::Medium => "medium",
                EncoderPreset::Slow => "slow",
                EncoderPreset::Slower => "slower",
                EncoderPreset::VerySlow => "veryslow",
            }
        }
    }

    // Change the function signature to not return anything
    fn bgra_to_yuv420p(&mut self, bgra: &[u8], width: u32, height: u32) {
        let y_size = (width * height) as usize;
        let u_size = y_size / 4;
        let v_size = y_size / 4;

        // Use pre-allocated buffer
        debug_assert!(self.yuv_buffer.len() >= y_size + u_size + v_size);

        // Y plane
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                let b = bgra[idx] as f32;
                let g = bgra[idx + 1] as f32;
                let r = bgra[idx + 2] as f32;

                let y_val = (0.257 * r + 0.504 * g + 0.098 * b + 16.0) as u8;
                self.yuv_buffer[(y * width + x) as usize] = y_val;
            }
        }

        // U plane
        for y in (0..height).step_by(2) {
            for x in (0..width).step_by(2) {
                let idx = ((y * width + x) * 4) as usize;
                let b = bgra[idx] as f32;
                let g = bgra[idx + 1] as f32;
                let r = bgra[idx + 2] as f32;

                let u_val = (-0.148 * r - 0.291 * g + 0.439 * b + 128.0) as u8;
                let u_idx = y_size + ((y / 2) * (width / 2) + (x / 2)) as usize;
                self.yuv_buffer[u_idx] = u_val;
            }
        }

        // V plane
        for y in (0..height).step_by(2) {
            for x in (0..width).step_by(2) {
                let idx = ((y * width + x) * 4) as usize;
                let b = bgra[idx] as f32;
                let g = bgra[idx + 1] as f32;
                let r = bgra[idx + 2] as f32;

                let v_val = (0.439 * r - 0.368 * g - 0.071 * b + 128.0) as u8;
                let v_idx = y_size + u_size + ((y / 2) * (width / 2) + (x / 2)) as usize;
                self.yuv_buffer[v_idx] = v_val;
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

            // Select encoder (hardware or software)
            let encoder_name = select_encoder_name(&self.config)?;
            info!("🎯 Selected encoder: {}", encoder_name);

            // Initialize FFmpeg
            ffmpeg::init()
                .map_err(|e| EncoderError::InitFailed(format!("FFmpeg init failed: {}", e)))?;

            let codec = ffmpeg::encoder::find_by_name(encoder_name)
                .ok_or_else(|| EncoderError::InitFailed(format!("Codec '{}' not found", encoder_name)))?;

            info!("Found codec: {}", codec.name());

            // Create encoder context
            let mut context = ffmpeg::codec::context::Context::new_with_codec(codec)
                .encoder()
                .video()
                .map_err(|e| EncoderError::InitFailed(format!("Failed to create encoder context: {}", e)))?;

            // Configure encoder
            context.set_width(self.config.width);
            context.set_height(self.config.height);
            context.set_format(ffmpeg::format::Pixel::YUV420P);
            context.set_time_base(ffmpeg::Rational::new(1, self.config.fps as i32));
            context.set_frame_rate(Some(ffmpeg::Rational::new(self.config.fps as i32, 1)));
            context.set_bit_rate(self.config.bitrate_kbps as usize * 1000);
            context.set_max_bit_rate(self.config.bitrate_kbps as usize * 1000);

            // Set GOP size (keyframe interval)
            context.set_gop(self.config.keyframe_interval);

            // Set preset
            let preset = self.preset_to_ffmpeg(encoder_name);
            info!("Using preset: {}", preset);

            // Set encoder-specific options
            let mut opts = ffmpeg::Dictionary::new();
            opts.set("preset", preset);

            if encoder_name.contains("nvenc") {
                info!("🚀 Configuring NVENC for low latency");
                // NVENC specific settings for low latency
                opts.set("tune", "ll"); // low latency
                opts.set("rc", "cbr"); // constant bitrate
                opts.set("cbr", "1");
                opts.set("delay", "0"); // no frame delay
                opts.set("zerolatency", "1");
                opts.set("forced-idr", "1");
                opts.set("gpu", "any");
                opts.set("repeat-headers", "1");
            } else if encoder_name.contains("x264") {
                info!("Configuring x264 for low latency");
                // x264 specific settings
                opts.set("tune", "zerolatency");
            }

            // Open encoder with options
            let opened_encoder = context
                .open_with(opts)
                .map_err(|e| EncoderError::InitFailed(format!("Failed to open encoder: {}", e)))?;

            self.encoder = Some(opened_encoder);
            self.initialized = true;

            info!("✅ FFmpeg encoder initialized successfully with {}", encoder_name);
            info!("   Resolution: {}x{}", self.config.width, self.config.height);
            info!("   FPS: {}", self.config.fps);
            info!("   Bitrate: {} kbps", self.config.bitrate_kbps);

            Ok(())
        }

        #[cfg(not(feature = "ffmpeg"))]
        Err(EncoderError::InitFailed("FFmpeg feature not enabled".into()))
    }

    fn encode(&mut self, frame: &RawFrame) -> Result<Option<EncodedPacket>> {
        #[cfg(feature = "ffmpeg")]
        {
            if !self.initialized {
                return Err(EncoderError::InitFailed("Encoder not initialized".into()));
            }

            // Convert to YUV if needed (mutably borrows self, but ends when done)
            if frame.format == PixelFormat::BGRA {
                self.bgra_to_yuv420p(&frame.data, frame.width, frame.height);
            }

            // Get config values we need (immutable borrow, ends immediately)
            let width = self.config.width;
            let height = self.config.height;
            let codec = self.config.codec;
            let frame_count = self.frame_count;

            let y_size = (width * height) as usize;
            let u_size = y_size / 4;

            // Get reference to YUV data
            let yuv_data = if frame.format == PixelFormat::BGRA {
                &self.yuv_buffer[..]
            } else {
                &frame.data[..]
            };

            // Create FFmpeg frame (no self access)
            let mut yuv_frame = ffmpeg::frame::Video::new(
                ffmpeg::format::Pixel::YUV420P,
                width,
                height,
            );

            // Copy planes
            yuv_frame.data_mut(0).copy_from_slice(&yuv_data[0..y_size]);
            yuv_frame.data_mut(1).copy_from_slice(&yuv_data[y_size..y_size + u_size]);
            yuv_frame.data_mut(2).copy_from_slice(&yuv_data[y_size + u_size..]);

            yuv_frame.set_pts(Some(frame_count as i64));

            // NOW borrow encoder (after all other self access is done)
            let encoder = self.encoder.as_mut()
                .ok_or_else(|| EncoderError::EncodingFailed("Encoder not available".into()))?;

            // Send frame to encoder
            encoder.send_frame(&yuv_frame)
                .map_err(|e| EncoderError::EncodingFailed(format!("Failed to send frame: {}", e)))?;

            // Receive encoded packet
            let mut encoded_packet = ffmpeg::codec::packet::Packet::empty();

            match encoder.receive_packet(&mut encoded_packet) {
                Ok(_) => {
                    let is_keyframe = encoded_packet.is_key();
                    let data = encoded_packet.data().unwrap_or(&[]).to_vec();

                    // Now we can access self again (encoder borrow ended)
                    self.frame_count += 1;

                    Ok(Some(EncodedPacket {
                        data: Bytes::from(data),
                        pts: frame.pts,
                        dts: frame.pts,
                        is_keyframe,
                        timestamp: frame.timestamp,
                        codec,
                    }))
                }
                Err(ffmpeg::Error::Other { errno: ffmpeg::error::EAGAIN }) => {
                    // Encoder needs more frames before producing output
                    self.frame_count += 1;
                    Ok(None)
                }
                Err(e) => {
                    Err(EncoderError::EncodingFailed(format!("Failed to receive packet: {}", e)))
                }
            }
        }

        #[cfg(not(feature = "ffmpeg"))]
        Err(EncoderError::EncodingFailed("FFmpeg feature not enabled".into()))
    }

    fn flush(&mut self) -> Result<Vec<EncodedPacket>> {
        #[cfg(feature = "ffmpeg")]
        {
            info!("Flushing FFmpeg encoder");

            if let Some(encoder) = &mut self.encoder {
                let mut packets = Vec::new();

                // Send flush signal
                if let Err(e) = encoder.send_eof() {
                    warn!("Failed to send EOF: {}", e);
                    return Ok(packets);
                }

                // Receive all remaining packets
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

                info!("Flushed {} packets", packets.len());
                return Ok(packets);
            }
        }

        Ok(vec![])
    }

    fn set_bitrate(&mut self, bitrate_kbps: u32) -> Result<()> {
        info!("Updating FFmpeg bitrate to {} kbps", bitrate_kbps);
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
            info!("Destroying FFmpeg encoder");
            let _ = self.flush();
            self.encoder = None;
        }
    }
}

fn select_encoder_name(config: &EncoderConfig) -> Result<&str> {
    // CRITICAL: Check hardware first, with detailed logging
    if config.use_hardware {
        match config.codec {
            VideoCodec::H264 => {
                if FfmpegEncoder::is_nvenc_available() {
                    info!("✅ NVENC H.264 encoder available - using hardware acceleration");
                    return Ok("h264_nvenc");
                } else {
                    error!("❌ NVENC not available! Reasons could be:");
                    error!("   - No NVIDIA GPU present");
                    error!("   - NVIDIA drivers not installed");
                    error!("   - FFmpeg not compiled with --enable-nvenc");
                    error!("   - GPU is too old (needs Kepler or newer)");
                    warn!("⚠️  Falling back to SOFTWARE encoding (will be VERY slow)");
                }
            }
            VideoCodec::H265 => {
                if FfmpegEncoder::is_nvenc_available() {
                    info!("✅ Using hevc_nvenc (NVIDIA hardware encoder)");
                    return Ok("hevc_nvenc");
                }
                warn!("NVENC not available for H.265, falling back to software");
            }
            _ => {}
        }
    } else {
        info!("Hardware encoding disabled in config - using software");
    }

    // Software fallback
    match config.codec {
        VideoCodec::H264 => {
            warn!("⚠️  Using libx264 SOFTWARE encoder - expect 200ms+ per frame!");
            warn!("⚠️  This is 100x slower than NVENC hardware encoding!");
            Ok("libx264")
        }
        VideoCodec::H265 => {
            warn!("⚠️  Using libx265 SOFTWARE encoder - will be extremely slow!");
            Ok("libx265")
        }
        _ => Err(EncoderError::UnsupportedCodec(config.codec)),
    }
}