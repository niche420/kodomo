#[cfg(all(target_os = "macos", feature = "videotoolbox"))]
pub mod videotoolbox {
    use super::*;

    pub struct VideoToolboxEncoder {
        config: EncoderConfig,
    }

    impl VideoToolboxEncoder {
        pub fn new(config: EncoderConfig) -> Result<Self> {
            // TODO: Initialize VideoToolbox compression session
            Err(EncoderError::HardwareUnavailable)
        }
    }

    impl VideoEncoder for VideoToolboxEncoder {
        fn init(&mut self, config: EncoderConfig) -> Result<()> {
            self.config = config;
            Ok(())
        }

        fn encode(&mut self, _frame: &RawFrame) -> Result<Option<EncodedPacket>> {
            Err(EncoderError::EncodingFailed("Not implemented".into()))
        }

        fn flush(&mut self) -> Result<Vec<EncodedPacket>> {
            Ok(vec![])
        }

        fn set_bitrate(&mut self, bitrate_kbps: u32) -> Result<()> {
            self.config.bitrate_kbps = bitrate_kbps;
            Ok(())
        }

        fn get_config(&self) -> &EncoderConfig {
            &self.config
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_factory() {
        let config = EncoderConfig::default();
        let encoder = EncoderFactory::create(config);
        assert!(encoder.is_ok());
    }

    #[test]
    fn test_software_encoder() {
        let config = EncoderConfig {
            width: 1280,
            height: 720,
            fps: 30,
            bitrate_kbps: 5000,
            codec: VideoCodec::H264,
            preset: EncoderPreset::Fast,
            keyframe_interval: 30,
            use_hardware: false,
        };

        let mut encoder = SoftwareEncoder::new(config).unwrap();
        encoder.init(encoder.config.clone()).unwrap();

        // Create a test frame
        let frame = RawFrame {
            data: vec![0u8; 1280 * 720 * 4],
            width: 1280,
            height: 720,
            stride: 1280 * 4,
            format: PixelFormat::BGRA,
            pts: 0,
            timestamp: Instant::now(),
        };

        let result = encoder.encode(&frame);
        assert!(result.is_ok());

        let packet = result.unwrap().unwrap();
        assert!(packet.is_keyframe); // First frame should be keyframe
        assert!(packet.size() > 0);
    }

    #[test]
    fn test_list_encoders() {
        let encoders = EncoderFactory::list_available_encoders();
        assert!(!encoders.is_empty());
        assert!(encoders[0].contains("Software"));
    }
}