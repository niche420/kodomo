pub mod packetizer;

use thiserror::Error;
use crate::error::{KdError, Result};

/// FU-A NAL type
const NAL_TYPE_FU_A: u8 = 28;

const NAL_TYPE_STAP_A: u8 = 24;

#[derive(Debug, Error, PartialEq)]
pub enum RtpError
{
    #[error("Buffer is size {0}")]
    BufferTooShort(usize),
    #[error("RTP version {0} is not supported")]
    UnsupportedVersion(u8),
    #[error("Expected sequence number {0}, got {1}")]
    UnexpectedSequenceNumber(u16, u16),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct RtpHeader
{
    marker: bool,
    payload_type: u8,
    sequence_number: u16,
    timestamp: u32,
    ssrc: u32
}

impl RtpHeader
{
    pub fn new(marker: bool, payload_type: u8, sequence_number: u16, timestamp: u32, ssrc: u32) -> RtpHeader {
        Self {
            marker,
            payload_type,
            sequence_number,
            timestamp,
            ssrc
        }
    }

    pub fn encode(&self) -> [u8; 12]
    {
        let mut data = [0u8; 12];
        data[0] = 0b1000_0000;
        data[1] = (self.marker as u8) << 7 | self.payload_type;
        data[2..4].copy_from_slice(&self.sequence_number.to_be_bytes());
        data[4..8].copy_from_slice(&self.timestamp.to_be_bytes());
        data[8..12].copy_from_slice(&self.ssrc.to_be_bytes());

        data
    }

    pub fn decode(data: &[u8]) -> Result<Self>
    {
        if data.len() < 12  {
            return Err(KdError::from(RtpError::BufferTooShort(data.len())));
        }

        let version = data[0] >> 6 & 0x03;
        if version != 2u8
        {
            return Err(KdError::from(RtpError::UnsupportedVersion(version)));
        }

        Ok(Self {
            marker: data[1] & 0b1000_0000 != 0,
            payload_type: data[1] & 0x7F,
            sequence_number: u16::from_be_bytes([data[2], data[3]]),
            timestamp: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
            ssrc: u32::from_be_bytes([data[8], data[9], data[10], data[11]]),
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct RtpPacket
{
    header: RtpHeader,
    payload: Vec<u8>,
}

impl RtpPacket
{
    pub fn new(header: RtpHeader, payload: Vec<u8>) -> Self
    {
        Self { header, payload }
    }

    pub fn encode(&self) -> Vec<u8>
    {
        let mut encoded = vec![];
        encoded.extend(self.header.encode());
        encoded.extend(self.payload.clone());

        encoded
    }

    pub fn decode(buf: &[u8]) -> Result<Self>
    {
        let header = RtpHeader::decode(buf)?;
        Ok(Self{
            header,
            payload: buf[12..].to_vec(),
        })
    }

    pub fn nal_type(&self) -> u8
    {
        self.payload[0] & 0x1F
    }

    pub fn parse(&self) -> Result<RtpPayload>
    {
        match self.nal_type()
        {
            NAL_TYPE_STAP_A => {
                // Account for type byte
                let mut nals = vec![];
                let payload_len = self.payload.len() - 1;
                let mut i = 1;
                while i < payload_len {
                    let nal_len = u16::from_be_bytes([self.payload[i], self.payload[i+1]]);
                    i += 2;
                    let nal = self.payload[i..i + nal_len as usize].to_vec();
                    nals.push(nal);
                    i += nal_len as usize;
                }
                Ok(RtpPayload::StapA {
                    nals
                })
            },
            NAL_TYPE_FU_A => {
                let indicator = self.payload[0];
                let header = self.payload[1];
                let fragment = self.payload[2..].to_vec();
                Ok(RtpPayload::FuA(FuAPayload{ indicator, header, fragment }))
            },
            _ => {
                Ok(RtpPayload::SingleNal {
                    nal: self.payload.clone()
                })
            }
        }
    }
}

#[derive(Debug)]
#[derive(PartialEq)]
pub enum RtpPayload {
    SingleNal {
        nal: Vec<u8>
    },
    FuA(FuAPayload),
    StapA {
        nals: Vec<Vec<u8>>
    }
}

#[derive(Debug, PartialEq)]
pub struct FuAPayload
{
    indicator: u8,
    header: u8,
    fragment: Vec<u8>
}

impl FuAPayload
{
    pub fn start(&self) -> bool
    {
        self.header & 0x80 == 0x80
    }

    pub fn end(&self) -> bool
    {
        self.header & 0x40 == 0x40
    }
}

#[derive(PartialEq)]
pub enum NalType
{
    Sps,
    Pps,
    Idr,
    PFrame,
    Other(u8)
}

impl From<u8> for NalType
{
    fn from(n: u8) -> Self {
        match n & 0x1F {
            1 => NalType::PFrame,
            5 => NalType::Idr,
            7 => NalType::Sps,
            8 => NalType::Pps,
            _ => NalType::Other(n)
        }
    }
}

#[cfg(test)] mod tests
{
    use crate::error::KdError;
    use crate::rtp::{FuAPayload, RtpError, RtpHeader, RtpPacket, RtpPayload, NAL_TYPE_FU_A, NAL_TYPE_STAP_A};

    #[test]
    fn test_zeroed_rtp_packet()
    {
        const MARKER: bool = false;
        const PAYLOAD_TYPE: u8 = 0;
        const SEQUENCE_NUMBER: u16 = 0;
        const TIMESTAMP: u32 = 0;
        const SSRC: u32 = 0;

        let header = RtpHeader::new(MARKER, PAYLOAD_TYPE, SEQUENCE_NUMBER, TIMESTAMP, SSRC);
        let payload = vec![0];
        let packet = RtpPacket::new(header, payload);
        let encoded = packet.encode();
        let decoded = RtpPacket::decode(&encoded).unwrap();
        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_filled_rtp_packet()
    {
        const MARKER: bool = true;
        const PAYLOAD_TYPE: u8 = 1;
        const SEQUENCE_NUMBER: u16 = 21;
        const TIMESTAMP: u32 = 67;
        const SSRC: u32 = 41;

        let header = RtpHeader::new(MARKER, PAYLOAD_TYPE, SEQUENCE_NUMBER, TIMESTAMP, SSRC);
        let payload = vec![0];
        let packet = RtpPacket::new(header, payload);
        let encoded = packet.encode();
        let decoded = RtpPacket::decode(&encoded).unwrap();
        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_buffer_too_short()
    {
        let data = [0u8; 11];
        let decoded = RtpPacket::decode(&data);
        assert!(matches!(decoded, Err(KdError::RtpError(RtpError::BufferTooShort(_)))));
    }

    #[test]
    fn test_unsupported_version()
    {
        let data = [0u8; 12];
        let decoded = RtpPacket::decode(&data);
        assert!(matches!(decoded, Err(KdError::RtpError(RtpError::UnsupportedVersion(_)))));
    }

    #[test]
    fn test_parse_single_nal()
    {
        const MARKER: bool = true;
        const PAYLOAD_TYPE: u8 = 1;
        const SEQUENCE_NUMBER: u16 = 21;
        const TIMESTAMP: u32 = 67;
        const SSRC: u32 = 41;

        let header = RtpHeader::new(MARKER, PAYLOAD_TYPE, SEQUENCE_NUMBER, TIMESTAMP, SSRC);
        let payload = vec![0];
        let packet = RtpPacket::new(header, payload.clone());
        let rtp_payload = packet.parse().unwrap();
        assert_eq!(rtp_payload, RtpPayload::SingleNal {
            nal: payload
        });
    }

    #[test]
    fn test_parse_fu_a()
    {
        const MARKER: bool = true;
        const PAYLOAD_TYPE: u8 = 1;
        const SEQUENCE_NUMBER: u16 = 21;
        const TIMESTAMP: u32 = 67;
        const SSRC: u32 = 41;

        let header = RtpHeader::new(MARKER, PAYLOAD_TYPE, SEQUENCE_NUMBER, TIMESTAMP, SSRC);
        let payload = vec![NAL_TYPE_FU_A, 21, 1, 2, 3];
        let packet = RtpPacket::new(header, payload.clone());
        let rtp_payload = packet.parse().unwrap();
        assert_eq!(rtp_payload, RtpPayload::FuA(
            FuAPayload {
                indicator: 28,
                header: 21,
                fragment: vec![1, 2, 3]
            }
        ));
    }

    #[test]
    fn test_parse_stap_a()
    {
        const MARKER: bool = true;
        const PAYLOAD_TYPE: u8 = 1;
        const SEQUENCE_NUMBER: u16 = 21;
        const TIMESTAMP: u32 = 67;
        const SSRC: u32 = 41;

        let header = RtpHeader::new(MARKER, PAYLOAD_TYPE, SEQUENCE_NUMBER, TIMESTAMP, SSRC);
        let payload = vec![NAL_TYPE_STAP_A, 0, 2, 1, 2, 0, 3, 1, 2, 3];
        let packet = RtpPacket::new(header, payload.clone());
        let rtp_payload = packet.parse().unwrap();
        assert_eq!(rtp_payload, RtpPayload::StapA {
            nals: vec![vec![1, 2], vec![1, 2, 3]]
        });
    }
}