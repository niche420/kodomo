use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::ptr;
use std::sync::Arc;
use tokio::sync::Mutex;

use kd_core::{StreamConfig, StreamingEngine};
use kd_capture::{CaptureConfig, PixelFormat as CapturePixelFormat};
use kd_encoder::{EncoderConfig, EncoderPreset, VideoCodec};
use kd_network::{NetworkConfig, TransportType};
use kd_input::{InputHandler, InputEvent, KeyboardEvent, MouseEvent};

// Initialize logging once
static INIT: std::sync::Once = std::sync::Once::new();

fn init_logging() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter("info")
            .init();
    });
}

// ============================================
// C-Compatible Types
// ============================================

/// Opaque handle to the streaming engine
pub struct StreamHandle {
    engine: Arc<Mutex<StreamingEngine>>,
    runtime: tokio::runtime::Runtime,
}

/// Error codes
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamError {
    Success = 0,
    InitError = 1,
    NotRunning = 2,
    AlreadyRunning = 3,
    InvalidConfig = 4,
    CaptureError = 5,
    EncodingError = 6,
    NetworkError = 7,
    InputError = 8,
    NullPointer = 9,
}

/// Video codec
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum StreamCodec {
    H264 = 0,
    H265 = 1,
    VP9 = 2,
}

impl From<StreamCodec> for VideoCodec {
    fn from(c: StreamCodec) -> Self {
        match c {
            StreamCodec::H264 => VideoCodec::H264,
            StreamCodec::H265 => VideoCodec::H265,
            StreamCodec::VP9 => VideoCodec::VP9,
        }
    }
}

/// Transport type
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum StreamTransport {
    WebRTC = 0,
    UDP = 1,
}

impl From<StreamTransport> for TransportType {
    fn from(t: StreamTransport) -> Self {
        match t {
            StreamTransport::WebRTC => TransportType::WebRTC,
            StreamTransport::UDP => TransportType::UDP,
        }
    }
}

/// Encoder preset
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum StreamPreset {
    UltraFast = 0,
    SuperFast = 1,
    VeryFast = 2,
    Faster = 3,
    Fast = 4,
    Medium = 5,
    Slow = 6,
}

impl From<StreamPreset> for EncoderPreset {
    fn from(p: StreamPreset) -> Self {
        match p {
            StreamPreset::UltraFast => EncoderPreset::UltraFast,
            StreamPreset::SuperFast => EncoderPreset::SuperFast,
            StreamPreset::VeryFast => EncoderPreset::VeryFast,
            StreamPreset::Faster => EncoderPreset::Faster,
            StreamPreset::Fast => EncoderPreset::Fast,
            StreamPreset::Medium => EncoderPreset::Medium,
            StreamPreset::Slow => EncoderPreset::Slow,
        }
    }
}

/// Configuration struct
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StreamingConfig {
    // Video settings
    pub width: c_uint,
    pub height: c_uint,
    pub fps: c_uint,
    pub bitrate_kbps: c_uint,
    pub codec: StreamCodec,
    pub preset: StreamPreset,
    pub hw_accel: c_int,

    // Network settings
    pub transport: StreamTransport,
    pub port: c_uint,

    // Input settings
    pub enable_keyboard: c_int,
    pub enable_mouse: c_int,
    pub enable_gamepad: c_int,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 60,
            bitrate_kbps: 10000,
            codec: StreamCodec::H264,
            preset: StreamPreset::Fast,
            hw_accel: 1,
            transport: StreamTransport::WebRTC,
            port: 8080,
            enable_keyboard: 1,
            enable_mouse: 1,
            enable_gamepad: 1,
        }
    }
}

impl From<StreamingConfig> for StreamConfig {
    fn from(c: StreamingConfig) -> Self {
        StreamConfig {
            video: kd_core::config::VideoConfig {
                width: c.width,
                height: c.height,
                fps: c.fps,
                bitrate_kbps: c.bitrate_kbps,
                codec: c.codec.into(),
                hw_accel: c.hw_accel != 0,
                keyframe_interval: c.fps, // 1 keyframe per second
            },
            audio: kd_core::config::AudioConfig {
                enabled: true,
                sample_rate: 48000,
                channels: 2,
                bitrate_kbps: 128,
            },
            network: kd_core::config::NetworkConfig {
                transport: c.transport.into(),
                port: c.port as u16,
                max_packet_size: 1400,
            },
            input: kd_core::config::InputConfig {
                keyboard_enabled: c.enable_keyboard != 0,
                mouse_enabled: c.enable_mouse != 0,
                gamepad_enabled: c.enable_gamepad != 0,
            },
        }
    }
}

/// Statistics
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StreamStats {
    pub frames_captured: u64,
    pub frames_encoded: u64,
    pub frames_sent: u64,
    pub frames_dropped: u64,
    pub bytes_sent: u64,
    pub average_fps: f64,
    pub average_bitrate_kbps: f64,
}

// ============================================
// Core Engine Functions
// ============================================

/// Initialize the streaming library (call once at startup)
#[unsafe(no_mangle)]
pub extern "C" fn stream_init() {
    init_logging();
    tracing::info!("Streaming library initialized");
}

/// Create a new streaming engine instance
///
/// # Safety
/// - config must be a valid pointer
/// - out_handle must be a valid pointer to store the result
/// - The returned handle must be freed with stream_destroy()
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stream_create(
    config: *const StreamingConfig,
    out_handle: *mut *mut StreamHandle,
) -> StreamError {
    if config.is_null() || out_handle.is_null() {
        return StreamError::NullPointer;
    }

    let rust_config: StreamConfig = (*config).into();

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!("Failed to create runtime: {}", e);
            return StreamError::InitError;
        }
    };

    let engine = match runtime.block_on(async {
        StreamingEngine::new(rust_config)
    }) {
        engine => Arc::new(Mutex::new(engine)),
    };

    let handle = Box::new(StreamHandle { engine, runtime });
    *out_handle = Box::into_raw(handle);

    tracing::info!("Streaming engine created");
    StreamError::Success
}

/// Get default configuration
#[unsafe(no_mangle)]
pub extern "C" fn stream_get_default_config() -> StreamingConfig {
    StreamingConfig::default()
}

/// Start the streaming engine
///
/// # Safety
/// - handle must be a valid pointer obtained from stream_create()
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stream_start(handle: *mut StreamHandle) -> StreamError {
    if handle.is_null() {
        return StreamError::NullPointer;
    }

    let handle = &*handle;
    match handle.runtime.block_on(async {
        handle.engine.lock().await.start().await
    }) {
        Ok(_) => {
            tracing::info!("Streaming started");
            StreamError::Success
        }
        Err(e) => {
            tracing::error!("Failed to start: {:?}", e);
            StreamError::InitError
        }
    }
}

/// Stop the streaming engine
///
/// # Safety
/// - handle must be a valid pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stream_stop(handle: *mut StreamHandle) -> StreamError {
    if handle.is_null() {
        return StreamError::NullPointer;
    }

    let handle = &*handle;
    match handle.runtime.block_on(async {
        handle.engine.lock().await.stop().await
    }) {
        Ok(_) => {
            tracing::info!("Streaming stopped");
            StreamError::Success
        }
        Err(e) => {
            tracing::error!("Failed to stop: {:?}", e);
            StreamError::NotRunning
        }
    }
}

/// Check if the engine is running
///
/// # Safety
/// - handle must be a valid pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stream_is_running(handle: *const StreamHandle) -> c_int {
    if handle.is_null() {
        return 0;
    }

    let handle = &*handle;
    handle.runtime.block_on(async {
        handle.engine.lock().await.is_running().await
    }) as c_int
}

/// Update configuration (can be called while running for dynamic changes)
///
/// # Safety
/// - handle must be a valid pointer
/// - config must be a valid pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stream_update_config(
    handle: *mut StreamHandle,
    config: *const StreamingConfig,
) -> StreamError {
    if handle.is_null() || config.is_null() {
        return StreamError::NullPointer;
    }

    let handle = &*handle;
    let rust_config: StreamConfig = (*config).into();

    match handle.runtime.block_on(async {
        handle.engine.lock().await.update_config(rust_config).await
    }) {
        Ok(_) => StreamError::Success,
        Err(_) => StreamError::InvalidConfig,
    }
}

/// Get statistics
///
/// # Safety
/// - handle must be a valid pointer
/// - out_stats must be a valid pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stream_get_stats(
    handle: *const StreamHandle,
    out_stats: *mut StreamStats,
) -> StreamError {
    if handle.is_null() || out_stats.is_null() {
        return StreamError::NullPointer;
    }

    let handle = &*handle;
    let stats = handle.runtime.block_on(async {
        handle.engine.lock().await.get_stats().await
    });

    *out_stats = StreamStats {
        frames_captured: stats.frames_captured,
        frames_encoded: stats.frames_encoded,
        frames_sent: stats.frames_sent,
        frames_dropped: stats.frames_dropped,
        bytes_sent: stats.bytes_sent,
        average_fps: 0.0, // TODO: Calculate
        average_bitrate_kbps: 0.0, // TODO: Calculate
    };

    StreamError::Success
}

/// Destroy the streaming engine and free resources
///
/// # Safety
/// - handle must be a valid pointer obtained from stream_create()
/// - handle must not be used after this call
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stream_destroy(handle: *mut StreamHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle);
        tracing::info!("Streaming engine destroyed");
    }
}

// ============================================
// Utility Functions
// ============================================

/// Get error message for an error code
///
/// # Safety
/// - The returned string is static and must not be freed
#[unsafe(no_mangle)]
pub extern "C" fn stream_error_string(error: StreamError) -> *const c_char {
    let msg = match error {
        StreamError::Success => "Success\0",
        StreamError::InitError => "Initialization error\0",
        StreamError::NotRunning => "Engine not running\0",
        StreamError::AlreadyRunning => "Engine already running\0",
        StreamError::InvalidConfig => "Invalid configuration\0",
        StreamError::CaptureError => "Screen capture error\0",
        StreamError::EncodingError => "Video encoding error\0",
        StreamError::NetworkError => "Network error\0",
        StreamError::InputError => "Input error\0",
        StreamError::NullPointer => "Null pointer\0",
    };

    msg.as_ptr() as *const c_char
}

/// Get library version
///
/// # Safety
/// - The returned string is static and must not be freed
#[unsafe(no_mangle)]
pub extern "C" fn stream_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

/// List available video encoders
///
/// # Safety
/// - out_count must be a valid pointer
/// - Returns array of strings that must be freed with stream_free_string_array()
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stream_list_encoders(out_count: *mut c_uint) -> *mut *mut c_char {
    if out_count.is_null() {
        return ptr::null_mut();
    }

    let encoders = kd_encoder::EncoderFactory::list_available_encoders();
    let count = encoders.len();

    let mut result = Vec::with_capacity(count);
    for encoder in encoders {
        if let Ok(c_str) = CString::new(encoder) {
            result.push(c_str.into_raw());
        }
    }

    *out_count = count as c_uint;

    let ptr = result.as_mut_ptr();
    std::mem::forget(result);
    ptr
}

/// Free string array returned by stream_list_encoders()
///
/// # Safety
/// - array must be obtained from stream_list_encoders()
/// - count must match the count returned by stream_list_encoders()
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stream_free_string_array(array: *mut *mut c_char, count: c_uint) {
    if array.is_null() {
        return;
    }

    for i in 0..count {
        let ptr = *array.add(i as usize);
        if !ptr.is_null() {
            let _ = CString::from_raw(ptr);
        }
    }

    let _ = Vec::from_raw_parts(array, count as usize, count as usize);
}

// ============================================
// Input Injection (Optional - for server use)
// ============================================

/// Send keyboard event to the system
///
/// # Safety
/// - key_code: Virtual key code (Windows VK_* compatible)
/// - is_pressed: 1 for key down, 0 for key up
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stream_inject_keyboard(
    key_code: c_uint,
    is_pressed: c_int,
) -> StreamError {
    // TODO: Implement keyboard injection
    tracing::debug!("Keyboard injection: key={}, pressed={}", key_code, is_pressed);
    StreamError::Success
}

/// Send mouse event to the system
///
/// # Safety
/// - x, y: Mouse coordinates
/// - button: Mouse button (0=none, 1=left, 2=right, 3=middle)
/// - is_pressed: 1 for button down, 0 for button up
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stream_inject_mouse(
    x: c_int,
    y: c_int,
    button: c_int,
    is_pressed: c_int,
) -> StreamError {
    // TODO: Implement mouse injection
    tracing::debug!("Mouse injection: pos=({},{}), button={}, pressed={}", 
                   x, y, button, is_pressed);
    StreamError::Success
}

// ============================================
// Callback Support (for events from Rust to C)
// ============================================

pub type FrameCallback = unsafe extern "C" fn(
    user_data: *mut c_void,
    frame_data: *const u8,
    width: c_uint,
    height: c_uint,
    timestamp: u64,
);

/// Set callback for captured frames (useful for debugging/recording)
///
/// # Safety
/// - handle must be a valid pointer
/// - callback can be null to disable
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stream_set_frame_callback(
    handle: *mut StreamHandle,
    callback: Option<FrameCallback>,
    user_data: *mut c_void,
) -> StreamError {
    if handle.is_null() {
        return StreamError::NullPointer;
    }

    // TODO: Store callback and invoke it when frames are captured
    tracing::debug!("Frame callback set");
    StreamError::Success
}