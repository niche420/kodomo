use serde::{Deserialize, Serialize};
use bytes::Bytes;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub msg_type: MessageType,
    pub payload: Vec<u8>,
    pub sequence: u64,
    pub timestamp_us: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    VideoFrame,
    AudioFrame,
    InputEvent,
    Control,
    Heartbeat,
    Acknowledgement,
}

impl Message {
    pub fn new(msg_type: MessageType, payload: Vec<u8>, sequence: u64) -> Self {
        Self {
            msg_type,
            payload,
            sequence,
            timestamp_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
        }
    }

    pub fn serialize(&self) -> Result<Bytes, Box<dyn std::error::Error>> {
        let config = bincode::config::standard();
        let encoded = bincode::serde::encode_to_vec(self, config)?;
        Ok(Bytes::from(encoded))
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let config = bincode::config::standard();
        let (msg, _): (Message, usize) = bincode::serde::decode_from_slice(data, config)?;
        Ok(msg)
    }
}