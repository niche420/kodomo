use std::collections::VecDeque;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use kd_shared::rtp::packetizer::Packetizer;
use kd_shared::rtp::{NalType, RtpPacket};
use kd_shared::profile::GameProfile;
use crate::{capture, encode};
use crate::capture::Frame;
use crate::encode::EncodeConfig;
use crate::network::NetworkConfig;
use crate::state::{ClientSession, SessionState};

// ─── Internal queue types ─────────────────────────────────────────────────────

#[derive(Default)]
struct FrameSlot(Mutex<Option<Frame>>);

impl FrameSlot {
    fn write(&self, frame: Frame) { *self.0.lock().unwrap() = Some(frame); }
    fn take(&self) -> Option<Frame> { self.0.lock().unwrap().take() }
}

struct PacketQueue {
    queue: Mutex<VecDeque<RtpPacket>>,
    has_packet: Condvar,
}

impl PacketQueue {
    const MAX: usize = 512;

    fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::with_capacity(Self::MAX)),
            has_packet: Condvar::new(),
        }
    }

    fn push(&self, packet: RtpPacket) {
        let q = &mut *self.queue.lock().unwrap();
        if q.len() == Self::MAX { q.pop_front(); }
        q.push_back(packet);
        self.has_packet.notify_one();
    }

    fn pop(&self, stopped: &AtomicBool) -> Option<RtpPacket> {
        let mut q = self.has_packet.wait_while(
            self.queue.lock().unwrap(),
            |q| q.is_empty() && !stopped.load(Ordering::SeqCst),
        ).unwrap();
        q.pop_front()
    }
}

// ─── Pipeline ─────────────────────────────────────────────────────────────────

pub struct Pipeline {
    frame_slot: Arc<FrameSlot>,
    packet_queue: Arc<PacketQueue>,
    client_ips: Arc<Mutex<Vec<IpAddr>>>,
    threads: Vec<JoinHandle<()>>,
    stopped: Arc<AtomicBool>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            frame_slot: Arc::new(FrameSlot::default()),
            packet_queue: Arc::new(PacketQueue::new()),
            client_ips: Arc::new(Mutex::new(Vec::new())),
            threads: vec![],
            stopped: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(
        &mut self,
        encode: &EncodeConfig,
        network: &NetworkConfig,
        session: SessionState,
    ) -> anyhow::Result<()> {
        self.stopped.store(false, Ordering::SeqCst);

        {
            let mut ips = self.client_ips.lock().unwrap();
            ips.clear();
            for client in &session.clients {
                if let Ok(ip) = IpAddr::from_str(&client.ip) {
                    ips.push(ip);
                }
            }
        }

        let slot = Arc::clone(&self.frame_slot);
        let stopped = Arc::clone(&self.stopped);
        let exe_path = session.exe_path.clone();
        self.spawn(move || capture_thread(exe_path, slot, stopped));

        let slot = Arc::clone(&self.frame_slot);
        let queue = Arc::clone(&self.packet_queue);
        let stopped = Arc::clone(&self.stopped);
        let encode = encode.clone();
        self.spawn(move || encode_thread(encode, slot, queue, stopped));

        let queue = Arc::clone(&self.packet_queue);
        let stopped = Arc::clone(&self.stopped);
        let client_ips = Arc::clone(&self.client_ips);
        let video_port = network.video_port;
        self.spawn(move || network_thread(client_ips, video_port, queue, stopped));

        for client in session.clients {
            self.spawn_input_thread(client.input_socket, client.profile);
        }

        Ok(())
    }

    pub fn add_client(&mut self, client: ClientSession) {
        if let Ok(ip) = IpAddr::from_str(&client.ip) {
            self.client_ips.lock().unwrap().push(ip);
        }
        self.spawn_input_thread(client.input_socket, client.profile);
    }

    pub fn stop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        self.packet_queue.has_packet.notify_all();
        self.threads.drain(..).for_each(|t| { t.join().ok(); });
        self.client_ips.lock().unwrap().clear();
    }

    fn spawn_input_thread(&mut self, socket: UdpSocket, profile: Option<GameProfile>) {
        let stopped = Arc::clone(&self.stopped);
        self.spawn(move || input_thread(socket, profile, stopped));
    }

    fn spawn<F: FnOnce() + Send + 'static>(&mut self, f: F) {
        self.threads.push(std::thread::spawn(f));
    }
}

// ─── Thread functions ─────────────────────────────────────────────────────────

fn capture_thread(exe_path: PathBuf, slot: Arc<FrameSlot>, stopped: Arc<AtomicBool>) {
    let mut capturer = match capture::create_capturer(&exe_path) {
        Ok(c) => c,
        Err(e) => { eprintln!("capture: {e}"); return; }
    };
    while !stopped.load(Ordering::SeqCst) {
        if let Some(frame) = capturer.capture_frame() {
            slot.write(frame);
        }
    }
}

fn encode_thread(
    config: EncodeConfig,
    slot: Arc<FrameSlot>,
    queue: Arc<PacketQueue>,
    stopped: Arc<AtomicBool>,
) {
    let mut encoder = match encode::create_encoder(&config) {
        Ok(e) => e,
        Err(e) => { eprintln!("encode: {e}"); return; }
    };
    let mut packetizer = Packetizer::new(0, 0, 1400);
    let mut timestamp: u32 = 0;

    while !stopped.load(Ordering::SeqCst) {
        if let Some(frame) = slot.take() {
            match encoder.encode_frame(frame) {
                Ok(nals) => {
                    let (sps_pps, rest): (Vec<_>, Vec<_>) = nals.iter().partition(|nal| {
                        matches!(NalType::from(nal[0]), NalType::Sps | NalType::Pps)
                    });
                    let rest: Vec<_> = rest.iter().filter(|nal| nal[0] & 0x1F != 6).collect();

                    if !sps_pps.is_empty() {
                        let slices: Vec<&[u8]> = sps_pps.iter().map(|n| n.as_slice()).collect();
                        queue.push(packetizer.packetize_stap_a(&slices, timestamp));
                    }
                    for (i, nal) in rest.iter().enumerate() {
                        let last = i == rest.len() - 1;
                        packetizer.packetize_nal(nal, timestamp, last)
                            .drain(..)
                            .for_each(|p| queue.push(p));
                    }
                }
                Err(e) => eprintln!("encode: {e}"),
            }
        }
        timestamp = timestamp.wrapping_add(90000 / 60);
    }
}

fn network_thread(
    client_ips: Arc<Mutex<Vec<IpAddr>>>,
    video_port: u16,
    queue: Arc<PacketQueue>,
    stopped: Arc<AtomicBool>,
) {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => { eprintln!("network: {e}"); return; }
    };

    while !stopped.load(Ordering::SeqCst) {
        let Some(packet) = queue.pop(&stopped) else { break; };
        let encoded = packet.encode();
        let ips = client_ips.lock().unwrap().clone();
        for ip in ips {
            let dest = SocketAddr::new(ip, video_port);
            if let Err(e) = socket.send_to(&encoded, dest) {
                eprintln!("network: {e}");
            }
        }
    }
}

fn input_thread(
    socket: UdpSocket,
    profile: Option<GameProfile>,
    stopped: Arc<AtomicBool>,
) {
    let Some(profile) = profile else {
        eprintln!("input: no active profile, thread exiting");
        return;
    };

    socket.set_read_timeout(Some(std::time::Duration::from_millis(100))).ok();

    let port = socket.local_addr().map(|a| a.port()).unwrap_or(0);
    eprintln!("input: listening on port {}", port);

    let mut injector = crate::input::create_injector();
    let mut buf = [0u8; 4096];

    while !stopped.load(Ordering::SeqCst) {
        match socket.recv(&mut buf) {
            Ok(len) => {
                match serde_json::from_slice::<kd_shared::profile::InputEvent>(&buf[..len]) {
                    Ok(event) => crate::input::dispatch(&mut *injector, &event, &profile),
                    Err(e) => eprintln!("input: parse error: {e}"),
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut => continue,
            Err(e) => eprintln!("input: {e}"),
        }
    }
}