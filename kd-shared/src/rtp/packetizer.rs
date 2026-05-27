use std::collections::BTreeMap;
use std::ops::Add;
use crate::rtp::{RtpError, RtpHeader, RtpPacket, RtpPayload, NAL_TYPE_FU_A, NAL_TYPE_STAP_A};
use crate::error::{KdError, Result};

pub struct Packetizer
{
    ssrc: u32,
    sequence_number: u16,
    mtu: usize,
}

impl Packetizer
{
    /// The RTP payload type for H.264
    const PAYLOAD_TYPE: u8 = 96;

    pub fn new(ssrc: u32, sequence_number: u16, mtu: usize) -> Packetizer
    {
        Self {
            ssrc,
            sequence_number,
            mtu
        }
    }

    pub fn packetize_nal(&mut self, nal: &[u8], timestamp: u32, marker: bool) -> Vec<RtpPacket>
    {
        if nal.len() <= self.mtu {
            self.packetize_single_nal(nal, timestamp, marker)
        } else {
            self.packetize_fu_a(nal, timestamp, marker)
        }
    }

    pub fn packetize_stap_a(&mut self, nals: &[&[u8]], timestamp: u32) -> RtpPacket
    {
        let header = RtpHeader::new(false, Self::PAYLOAD_TYPE, self.sequence_number, timestamp, self.ssrc);
        let mut payload = vec![NAL_TYPE_STAP_A];
        nals.iter().for_each(|nal|{
            let nal_len = (nal.len() as u16).to_be_bytes();
            payload.append(&mut nal_len.to_vec());
            payload.extend_from_slice(&nal);
        });
        let packet = RtpPacket::new(header, payload);
        self.sequence_number = self.sequence_number.wrapping_add(1);

        packet
    }

    fn packetize_single_nal(&mut self, nal: &[u8], timestamp: u32, marker: bool) -> Vec<RtpPacket>
    {
        let header = RtpHeader::new(marker, Self::PAYLOAD_TYPE, self.sequence_number, timestamp, self.ssrc);
        self.sequence_number = self.sequence_number.wrapping_add(1);
        vec![RtpPacket::new(header, Vec::from(nal))]
    }

    fn packetize_fu_a(&mut self, nal: &[u8], timestamp: u32, marker: bool) -> Vec<RtpPacket>
    {
        let nal_header = nal[0];
        let f = nal_header & 0x80;
        let nri = nal_header & 0x60;
        let nal_type = nal_header & 0x1F;
        let fua_indicator = f | nri | NAL_TYPE_FU_A;
        // Split into MTU - 2 sized chunks (Each chunk has the FU-A indicator and header bytes)
        let chunks = nal[1..].chunks(self.mtu - 2);
        let num_chunks = chunks.len();
        let mut fragments = Vec::with_capacity(chunks.len());
        for (idx, chunk) in chunks.enumerate() {
            let s = ((idx == 0) as u8) << 7;
            let e = ((idx == num_chunks - 1) as u8) << 6;
            let fua_header = s | e | nal_type;
            let header = RtpHeader::new(if e != 0 { marker } else { false }, Self::PAYLOAD_TYPE, self.sequence_number, timestamp, self.ssrc);
            let mut payload = vec![fua_indicator, fua_header];
            payload.extend(chunk);
            fragments.push(RtpPacket::new(header, payload));
            self.sequence_number = self.sequence_number.wrapping_add(1);
        }

        fragments
    }
}

pub struct Depacketizer
{
    fu_a_buffer: Vec<u8>,
    is_assembling: bool,
    expected_sequence_number: u16,
    reorder_buffer: BTreeMap<u16, RtpPacket>,
}

impl Depacketizer
{
    pub fn new() -> Depacketizer {
        Self {
            fu_a_buffer: vec![],
            is_assembling: false,
            expected_sequence_number: 0,
            reorder_buffer: BTreeMap::new(),
        }
    }

    pub fn push(&mut self, packet: RtpPacket) -> Result<Option<Vec<Vec<u8>>>>
    {
        eprintln!("Inserting seq={}, expected={}, buffer_size={}",
                  packet.header.sequence_number,
                  self.expected_sequence_number,
                  self.reorder_buffer.len());

        if !self.reorder_buffer.is_empty() || self.is_assembling {
            // drop it, we're mid-stream
        } else {
            // reset to this packet
            self.expected_sequence_number = packet.header.sequence_number;
        }

        self.reorder_buffer.insert(packet.header.sequence_number, packet);

        let mut all_nals = vec![];
        while let Some(packet) = self.reorder_buffer.remove(&self.expected_sequence_number) {
            self.expected_sequence_number = self.expected_sequence_number.wrapping_add(1);
            self.process_packet(packet, &mut all_nals)?;
        }

        if self.reorder_buffer.len() > 8 {
            self.expected_sequence_number = *self.reorder_buffer.keys().next().unwrap();
            self.fu_a_buffer.clear();
            self.is_assembling = false;
        }

        if all_nals.is_empty() { Ok(None) } else { Ok(Some(all_nals)) }
    }

    fn process_packet(&mut self, packet: RtpPacket, all_nals: &mut Vec<Vec<u8>>) -> Result<()>
    {
        let payload = packet.parse()?;
        match payload
        {
            RtpPayload::SingleNal { nal } => {
                all_nals.push(nal);
            },
            RtpPayload::StapA { nals } => {
                all_nals.extend(nals);
            },
            RtpPayload::FuA(payload) => {
                if payload.start() {
                    self.fu_a_buffer.clear();
                    self.is_assembling = true;
                    self.fu_a_buffer.push(payload.indicator & 0xE0 | payload.header & 0x1F);
                    self.fu_a_buffer.extend(payload.fragment);
                }
                else if payload.end() {
                    self.fu_a_buffer.extend(payload.fragment);
                    self.is_assembling = false;
                    all_nals.push(self.fu_a_buffer.clone());
                }
                else {
                    self.fu_a_buffer.extend(payload.fragment);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)] mod tests
{
    use crate::error::KdError;
    use crate::rtp::{RtpError, RtpHeader, RtpPacket, NAL_TYPE_FU_A, NAL_TYPE_STAP_A};
    use crate::rtp::packetizer::{Depacketizer, Packetizer};

    #[test]
    fn test_packetize_single_nal()
    {
        let mut tizer = Packetizer::new(1, 0, 4);
        let nal_header: u8 = 0b0111_1111;
        let nal = [nal_header, 0b0];
        let packets = tizer.packetize_nal(&nal, 21, false);

        assert_eq!(1, packets.len());
        assert_eq!(0, packets[0].header.sequence_number);
    }

    #[test]
    fn test_packetize_nal_fu_a()
    {
        let mut tizer = Packetizer::new(1, 0, 4);
        let nal_header: u8 = 0b0111_1111;
        let nal = [nal_header, 0xFF, 0xFF, 0xFF, 0xFF];
        let packets = tizer.packetize_nal(&nal, 21, false);

        assert!(packets.len() > 1);
        assert_eq!(packets[0].payload[1] & 0x80, 0x80);
        assert_eq!(packets.last().unwrap().payload[1] & 0x40, 0x40);
    }

    #[test]
    fn test_middle_packets_s_e_bits()
    {
        let mut tizer = Packetizer::new(1, 0, 4);
        let nal_header: u8 = 0b0111_1111;
        let mut nal = vec![0xFF; 24];
        nal.insert(0, nal_header);
        let packets = tizer.packetize_nal(&nal, 21, false);
        packets[1..packets.len()-1].iter().for_each(|packet| {
            assert_ne!(packet.payload[1] & 0x80, 0x80);
            assert_ne!(packet.payload[1] & 0x40, 0x40);
        });
    }

    #[test]
    fn test_marker()
    {
        let mut tizer = Packetizer::new(1, 0, 4);
        let nal_header: u8 = 0b0111_1111;
        let mut nal = vec![0xFF; 24];
        nal.insert(0, nal_header);
        let packets = tizer.packetize_nal(&nal, 21, true);
        packets[0..packets.len()-1].iter().for_each(|packet| {
            assert!(!packet.header.marker);
        });
        assert!(packets.last().unwrap().header.marker);
    }

    #[test]
    fn test_sequence_number()
    {
        let mut sequence_number = 15;
        let mut tizer = Packetizer::new(1, sequence_number, 4);
        let nal_header: u8 = 0b0111_1111;
        let mut nal = vec![0xFF; 24];
        nal.insert(0, nal_header);
        let packets = tizer.packetize_nal(&nal, 21, true);
        packets.iter().for_each(|packet| {
            assert_eq!(packet.header.sequence_number, sequence_number);
            sequence_number = sequence_number.wrapping_add(1);
        });
    }

    #[test]
    fn test_fua_type()
    {
        let mut tizer = Packetizer::new(1, 0, 4);
        let nal_header: u8 = 0b0111_1111;
        let mut nal = vec![0xFF; 24];
        nal.insert(0, nal_header);
        let packets = tizer.packetize_nal(&nal, 21, true);
        packets.iter().for_each(|packet| {
            assert_eq!(packet.payload[0] & 0x1F, NAL_TYPE_FU_A);
        });
    }

    #[test]
    fn test_nal_type()
    {
        let mut tizer = Packetizer::new(1, 0, 4);
        let nal_header: u8 = 0b0111_1111;
        let mut nal = vec![0xFF; 24];
        nal.insert(0, nal_header);
        let packets = tizer.packetize_nal(&nal, 21, true);
        packets.iter().for_each(|packet| {
            assert_eq!(packet.payload[1] & 0x1F, nal_header & 0x1F);
        });
    }

    #[test]
    fn test_stap_a_type()
    {
        let mut tizer = Packetizer::new(1, 0, 4);
        let nal_header: u8 = 0b0111_1111;
        let mut nal = vec![0xFF; 24];
        nal.insert(0, nal_header);
        let packet = tizer.packetize_stap_a(&[&nal], 21);
        assert_eq!(packet.payload[0], NAL_TYPE_STAP_A);
    }

    #[test]
    fn test_stap_a_size_bytes()
    {
        let mut tizer = Packetizer::new(1, 0, 4);
        let nal_header: u8 = 0b0111_1111;
        let mut nal = vec![0xFF; 3];
        nal.insert(0, nal_header);
        let packet = tizer.packetize_stap_a(&[&nal], 21);
        let nal_len = u16::from_be_bytes([packet.payload[1], packet.payload[2]]);
        assert_eq!(nal_len, 4);
    }

    #[test]
    fn test_stap_a_marker_false()
    {
        let mut tizer = Packetizer::new(1, 0, 8);
        let nal_header: u8 = 0b0111_1111;
        let mut nal1 = vec![0xFF; 3];
        nal1.insert(0, nal_header);
        let mut nal2 = vec![0xFF; 3];
        nal2.insert(0, nal_header);
        let packet = tizer.packetize_stap_a(&[&nal1, &nal2], 21);
        assert!(!packet.header.marker);
    }

    #[test]
    fn test_stap_a_seq_num_inc()
    {
        let mut tizer = Packetizer::new(1, 0, 8);
        let nal_header: u8 = 0b0111_1111;
        let mut nal1 = vec![0xFF; 3];
        nal1.insert(0, nal_header);
        let mut nal2 = vec![0xFF; 3];
        nal2.insert(0, nal_header);
        let packet = tizer.packetize_stap_a(&[&nal1, &nal2], 21);
        assert_eq!(packet.header.sequence_number, 0);
        assert_eq!(tizer.sequence_number, 1);
    }

    #[test]
    fn test_stap_a_nal_order()
    {
        let mut tizer = Packetizer::new(1, 0, 8);
        let nal_header: u8 = 0b0111_1111;
        let mut nal1 = vec![0xFF; 3];
        nal1.insert(0, nal_header);
        let mut nal2 = vec![0xFF; 3];
        nal2.insert(0, nal_header);
        let packet = tizer.packetize_stap_a(&[&nal1, &nal2], 21);
        assert_eq!(packet.payload[3..7], nal1);
        assert_eq!(packet.payload[9..13], nal2);
    }

    #[test]
    fn test_single_nal_roundtrip()
    {
        let mut packetizer = Packetizer::new(1, 0, 8);
        let mut depacketizer = Depacketizer::new();
        let nal = vec![0xFF; 8];
        let packets = packetizer.packetize_single_nal(&nal, 21, false);
        let nals = depacketizer.push(packets[0].clone()).unwrap();
        assert_eq!(nals, Some(vec![nal]));
    }

    #[test]
    fn test_fu_a_roundtrip()
    {
        let mut packetizer = Packetizer::new(1, 0, 8);
        let mut depacketizer = Depacketizer::new();
        let nal = vec![0xFF; 8];
        let packets = packetizer.packetize_fu_a(&nal, 21, false);
        let mut nals = depacketizer.push(packets[0].clone()).unwrap();
        assert_eq!(nals, None);
        nals = depacketizer.push(packets[1].clone()).unwrap();
        assert_eq!(nals, Some(vec![nal]));
    }

    #[test]
    fn test_stap_a_roundtrip()
    {
        let mut packetizer = Packetizer::new(1, 0, 8);
        let mut depacketizer = Depacketizer::new();
        let nal1 = vec![0xFF; 4];
        let nal2 = vec![0xCF; 4];
        let packet = packetizer.packetize_stap_a(&[&nal1, &nal2], 21);
        let mut nals = depacketizer.push(packet).unwrap();
        assert_eq!(nals, Some(vec![nal1, nal2]));
    }

    #[test]
    fn test_fu_a_lost_fragment()
    {
        let mut packetizer = Packetizer::new(1, 0, 8);
        let mut depacketizer = Depacketizer::new();
        let nal = vec![0xFF; 8];
        let mut packets = packetizer.packetize_fu_a(&nal, 21, false);
        let nals = depacketizer.push(packets[0].clone()).unwrap();
        assert_eq!(nals, None);
        packets[1].header.sequence_number = 0xFF;
        let corrupted = depacketizer.push(packets[1].clone());
        assert_eq!(corrupted, Ok(None));
    }
}