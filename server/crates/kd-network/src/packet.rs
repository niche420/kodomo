use bytes::{Bytes, BytesMut, Buf, BufMut};

/// Wire format packet structure
#[derive(Debug, Clone)]
pub struct Packet {
    pub packet_type: PacketType,
    pub sequence: u32,
    pub timestamp: u64,
    pub flags: u8,
    pub payload: Bytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketType {
    Video = 0x01,
    Audio = 0x02,
    Input = 0x03,
    Control = 0x04,
}

// Packet flags
pub const FLAG_KEYFRAME: u8 = 0x01;
pub const FLAG_FRAGMENT: u8 = 0x02;
pub const FLAG_LAST_FRAGMENT: u8 = 0x04;

impl Packet {
    pub fn new(packet_type: PacketType, sequence: u32, payload: Bytes) -> Self {
        Self {
            packet_type,
            sequence,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            flags: 0,
            payload,
        }
    }

    pub fn with_flags(mut self, flags: u8) -> Self {
        self.flags = flags;
        self
    }

    /// Serialize packet to wire format
    /// Format: [type:1][seq:4][timestamp:8][flags:1][payload_len:4][payload:N]
    pub fn to_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(18 + self.payload.len());

        buf.put_u8(self.packet_type as u8);
        buf.put_u32(self.sequence);
        buf.put_u64(self.timestamp);
        buf.put_u8(self.flags);
        buf.put_u32(self.payload.len() as u32);
        buf.put(self.payload.clone());

        buf.freeze()
    }

    /// Deserialize packet from wire format
    pub fn from_bytes(mut data: Bytes) -> Result<Self, String> {
        if data.len() < 18 {
            return Err("Packet too short".into());
        }

        let packet_type = match data.get_u8() {
            0x01 => PacketType::Video,
            0x02 => PacketType::Audio,
            0x03 => PacketType::Input,
            0x04 => PacketType::Control,
            _ => return Err("Invalid packet type".into()),
        };

        let sequence = data.get_u32();
        let timestamp = data.get_u64();
        let flags = data.get_u8();
        let payload_len = data.get_u32() as usize;

        if data.len() < payload_len {
            return Err("Incomplete payload".into());
        }

        let payload = data.split_to(payload_len);

        Ok(Self {
            packet_type,
            sequence,
            timestamp,
            flags,
            payload,
        })
    }

    pub fn is_keyframe(&self) -> bool {
        (self.flags & FLAG_KEYFRAME) != 0
    }
}