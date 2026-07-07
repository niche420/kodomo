use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use kd_shared::rtp::packetizer::Packetizer;
use kd_shared::rtp::NalType;
use crate::encode;
use crate::encode::EncodeConfig;
use crate::session::common::{FrameSlot, PacketQueue};
use crate::session::SessionWorker;

pub struct EncodeWorker {
    config: EncodeConfig,
    frame_slot: Arc<FrameSlot>,
    packet_queue: Arc<PacketQueue>,
    stopped: Arc<AtomicBool>
}

impl EncodeWorker {
    pub fn new(config: EncodeConfig, frame_slot: Arc<FrameSlot>, packet_queue: Arc<PacketQueue>, stopped: Arc<AtomicBool>) -> EncodeWorker {
        Self { config, frame_slot, packet_queue, stopped }
    }
}

impl SessionWorker for EncodeWorker {
    fn run(&mut self) {
        let mut encoder = match encode::create_encoder(&self.config) {
            Ok(e) => e,
            Err(e) => { eprintln!("encode: {e}"); return; }
        };
        let mut packetizer = Packetizer::new(0, 0, 1400);
        let mut timestamp: u32 = 0;

        while !self.stopped.load(Ordering::SeqCst) {
            if let Some(frame) = self.frame_slot.take() {
                match encoder.encode_frame(frame) {
                    Ok(nals) => {
                        let (sps_pps, rest): (Vec<_>, Vec<_>) = nals.iter().partition(|nal| {
                            matches!(NalType::from(nal[0]), NalType::Sps | NalType::Pps)
                        });
                        let rest: Vec<_> = rest.iter().filter(|nal| nal[0] & 0x1F != 6).collect();

                        if !sps_pps.is_empty() {
                            let slices: Vec<&[u8]> = sps_pps.iter().map(|n| n.as_slice()).collect();
                            self.packet_queue.push(packetizer.packetize_stap_a(&slices, timestamp));
                        }
                        for (i, nal) in rest.iter().enumerate() {
                            let last = i == rest.len() - 1;
                            packetizer.packetize_nal(nal, timestamp, last)
                                .drain(..)
                                .for_each(|p| self.packet_queue.push(p));
                        }
                    }
                    Err(e) => eprintln!("encode: {e}"),
                }
            }
            timestamp = timestamp.wrapping_add(90000 / 60);
        }
    }
}