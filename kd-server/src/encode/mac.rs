use crate::capture::Frame;
use crate::encode::FrameEncoder;

pub struct VideoToolboxEncoder {

}

impl VideoToolboxEncoder {
    pub fn new(frame_width: u32, frame_height: u32, fps: u32) -> anyhow::Result<Self> {
        Ok(Self {})
    }
}

impl FrameEncoder for VideoToolboxEncoder {
    fn encode_frame(&mut self, frame: Frame) -> anyhow::Result<Vec<Vec<u8>>> {
        todo!()
    }
}