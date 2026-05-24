use std::collections::VecDeque;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::str::FromStr;
use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use serde::{Deserialize, Serialize};
use kd_shared::rtp::packetizer::Packetizer;
use kd_shared::rtp::{NalType, RtpPacket};
use crate::{capture, encode};
use crate::capture::{Frame, FrameCapturer};
use crate::encode::{EncodeConfig, FrameEncoder};
use crate::network::NetworkConfig;

#[derive(Default)]
struct FrameSlot(Mutex<Option<Frame>>);

impl FrameSlot {
    pub fn new() -> Self {
        FrameSlot(Mutex::new(None))
    }

    pub fn write(&self, frame: Frame) {
        *self.0.lock().unwrap() = Some(frame);
    }

    pub fn take(&self) -> Option<Frame> {
        self.0.lock().unwrap().take()
    }
}

#[derive(Default)]
struct PacketQueue
{
    queue: Mutex<VecDeque<RtpPacket>>,
    has_packet: Condvar
}

impl PacketQueue {
    const MAX_PACKETS: usize = 8;

    pub fn new() -> PacketQueue {
        Self {
            queue: Mutex::new(VecDeque::with_capacity(Self::MAX_PACKETS)),
            has_packet: Condvar::new()
        }
    }

    pub fn push(&self, packet: RtpPacket) {
        let queue = &mut *self.queue.lock().unwrap();
        if queue.len() == Self::MAX_PACKETS {
            queue.pop_front();
        }
        queue.push_back(packet);
        self.has_packet.notify_one();
    }

    pub fn pop(&self) -> RtpPacket {
        let mut queue = &mut *self.has_packet.wait_while(
            self.queue.lock().unwrap(),
            |q| q.is_empty()
        ).unwrap();
        queue.pop_front().unwrap()
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct PipelineConfig {
    encode: EncodeConfig,
    network: NetworkConfig,
}

#[derive(Default)]
pub struct Pipeline {
    frame_slot: Arc<FrameSlot>,
    packet_queue: Arc<PacketQueue>,
    threads: Vec<JoinHandle<()>>,
    stopped: Arc<AtomicBool>,
}

impl Pipeline {
    pub fn new() -> Pipeline {
        Self {
            frame_slot: Arc::new(FrameSlot::new()),
            packet_queue: Arc::new(PacketQueue::new()),
            threads: vec![],
            stopped: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&mut self, config: PipelineConfig) -> anyhow::Result<()> {
        let slot = Arc::clone(&self.frame_slot);
        let stopped = Arc::clone(&self.stopped);
        self.spawn_stage_thread(move || capture_thread(slot, stopped));

        let slot = Arc::clone(&self.frame_slot);
        let stopped = Arc::clone(&self.stopped);
        let queue = Arc::clone(&self.packet_queue);
        self.spawn_stage_thread(move || encode_thread(&config.encode, slot, queue, stopped));

        let slot = Arc::clone(&self.frame_slot);
        let stopped = Arc::clone(&self.stopped);
        let queue = Arc::clone(&self.packet_queue);
        self.spawn_stage_thread(move || network_thread(&config.network, queue, stopped));

        Ok(())
    }

    pub fn stop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        self.threads.drain(..).for_each(|t| t.join().unwrap());
    }

    fn spawn_stage_thread<F>(&mut self, stage: F)
    where
        F: FnOnce() + Send + 'static {
        let handle = std::thread::spawn(move || {
            stage();
        });
        self.threads.push(handle);
    }
}

fn capture_thread(slot: Arc<FrameSlot>, stopped: Arc<AtomicBool>) {
    let mut capturer = match capture::create_capturer() {
        Ok(capturer) => capturer,
        Err(e) => {
            eprintln!("Capture error: {e}");
            return;
        }
    };
    
    while !stopped.load(Ordering::SeqCst) {
        match capturer.capture_frame() {
            Ok(frame) => slot.write(frame),
            Err(e) => eprintln!("Capture error: {e}"),
        }
    }
}

fn encode_thread(config: &EncodeConfig, slot: Arc<FrameSlot>, queue: Arc<PacketQueue>, stopped: Arc<AtomicBool>) {
    let mut encoder = match encode::create_encoder(&config) {
        Ok(encoder) => encoder,
        Err(e) => {
            eprintln!("Encode error: {e}");
            return;
        }
    };
    let mut packetizer = Packetizer::new(0, 0, 1400);
    let mut timestamp: u32 = 0;

    while !stopped.load(Ordering::SeqCst) {
        match slot.take() {
            Some(frame) => {
                match encoder.encode_frame(frame) {
                    Ok(nals) => {
                        let (sps_pps, rest): (Vec<_>, Vec<_>) = nals.iter().partition(|nal| {
                            matches!(NalType::from(nal[0]), NalType::Sps | NalType::Pps)
                        });

                        if !sps_pps.is_empty() {
                            let sps_pps_slice: Vec<&[u8]> = sps_pps.iter().map(|nal| nal.as_slice()).collect();
                            let packet = packetizer.packetize_stap_a(&sps_pps_slice, timestamp);
                            queue.push(packet);
                        }

                        for (i, nal) in rest.iter().enumerate() {
                            let mut packets = packetizer.packetize_nal(&nal, timestamp, i == rest.len() - 1);
                            packets.drain(..).for_each(|packet| {queue.push(packet);});
                        }
                    },
                    Err(e) => eprintln!("Encode error: {e}")
                }
            }
            None => {}
        }

        timestamp = timestamp.wrapping_add(90000 / 60);
    }
}

fn network_thread(config: &NetworkConfig, queue: Arc<PacketQueue>, stopped: Arc<AtomicBool>) {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Network error: {e}");
            return;
        }
    };
    let dest_ip = match IpAddr::from_str(&*config.dest_ip) {
        Ok(ip) => ip,
        Err(e) => {
            eprintln!("Network error: {e}");
            return;
        }
    };
    let dest = SocketAddr::new(dest_ip, config.video_port);

    while !stopped.load(Ordering::SeqCst) {
        let packet = queue.pop();
        match socket.send_to(&packet.encode(), dest) {
            Ok(_) => {},
            Err(e) => eprintln!("Network error: {e}")
        }
    }
}