mod win32;

#[cfg(feature = "nvenc")]
use nvenc as nv;
use nvenc::session::Session;
use crate::{EncodedPacket, EncoderConfig, RawFrame, VideoEncoder};

pub struct NvEncoder {
    config: EncoderConfig,
}

impl NvEncoder {
    pub fn new(config: EncoderConfig) -> Self {
        Self { config }
    }
}

impl VideoEncoder for NvEncoder {
    fn init(&mut self, config: EncoderConfig) -> crate::Result<()> {
        #[cfg(windows)]
        {
            
            //let session: Session<nv::session::NeedsConfig> = Session::open_dx(&device).unwrap();
        }
        
        Ok(())
    }

    fn encode(&mut self, frame: &RawFrame) -> crate::Result<Option<EncodedPacket>> {
        todo!()
    }

    fn flush(&mut self) -> crate::Result<Vec<EncodedPacket>> {
        todo!()
    }

    fn set_bitrate(&mut self, bitrate_kbps: u32) -> crate::Result<()> {
        todo!()
    }

    fn get_config(&self) -> &EncoderConfig {
        todo!()
    }
}