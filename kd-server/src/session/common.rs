use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use kd_shared::rtp::RtpPacket;
use crate::capture::Frame;

#[derive(Default)]
pub struct FrameSlot(Mutex<Option<Frame>>);

impl FrameSlot {
    pub fn write(&self, frame: Frame) { *self.0.lock().unwrap() = Some(frame); }
    pub fn take(&self) -> Option<Frame> { self.0.lock().unwrap().take() }
}

pub struct PacketQueue {
    queue: Mutex<VecDeque<RtpPacket>>,
    has_packet: Condvar,
}

impl PacketQueue {
    const MAX: usize = 512;

    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::with_capacity(Self::MAX)),
            has_packet: Condvar::new(),
        }
    }

    pub fn push(&self, packet: RtpPacket) {
        let q = &mut *self.queue.lock().unwrap();
        if q.len() == Self::MAX { q.pop_front(); }
        q.push_back(packet);
        self.has_packet.notify_one();
    }

    pub fn pop(&self) -> Option<RtpPacket> {
        let mut q = self.has_packet.wait_while(
            self.queue.lock().unwrap(),
            |q| q.is_empty(),
        ).unwrap();
        q.pop_front()
    }
    
    pub fn reset(&self) {
        self.has_packet.notify_all();
    }
}