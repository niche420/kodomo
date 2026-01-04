use std::cell::RefCell;
use crate::{CaptureHandler, PixelFormat, Result};
use std::mem;
use std::ptr;
use std::rc::Rc;
use std::sync::{atomic, Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tracing::{debug, error, info};
use windows::{
    core::*,
    Win32::Graphics::{
        Direct3D11::*,
        Dxgi::{Common::*, *},
    },
};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::{HMODULE, LPARAM, WPARAM};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess;
use windows::Win32::System::Threading::{GetCurrentThreadId, GetThreadId};
use windows::Win32::UI::WindowsAndMessaging::{DispatchMessageW, PeekMessageW, PostThreadMessageW, TranslateMessage, PM_REMOVE, WM_QUIT};
use crate::{CaptureConfig, CaptureError, CapturedFrame, MonitorInfo, ScreenCapture};
use crate::win32::{d3d11, frame_pool, window};
use crate::win32::d3d11::{Device, StagingTexture};
use crate::win32::frame_pool::{FramePool};
use crate::win32::window::Window;

pub type EventCallbackToken = i64;
pub type EventCallbackFn<Sender> = TypedEventHandler::<Sender, IInspectable>;

pub fn start_window_capture<H: CaptureHandler>(
    window_name: String,
    should_stop: &Arc<AtomicBool>,
    handler: Arc<Mutex<H>>,
) -> Result<()> {
    info!("Initializing Graphics Capture for window: {}", window_name);

    // Initialize D3D11 device
    let device = Device::new()?;

    // Find window
    let window = Window::from_contains_name(&window_name)?;
    let item: GraphicsCaptureItem = window.try_into()?;

    // Get initial size
    let size = item.Size()?;
    info!("Window size: {}x{}", size.Width, size.Height);

    // Create staging texture for CPU readback
    let staging = Arc::new(StagingTexture::new(
        device.device(),
        size.Width as u32,
        size.Height as u32,
        windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
    )?);

    // Create frame pool using your existing infrastructure
    let mut frame_pool = FramePool::new(
        &device,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        &item,
    )?;

    let should_stop_frame = should_stop.clone();

    // Set frame arrived on your FramePool
    frame_pool.set_frame_arrived(on_frame_arrived(should_stop.clone(), staging, handler.clone()))?;

    let closed_token = item.Closed(&on_capture_closed(should_stop.clone(), handler))?;

    frame_pool.start()?;
    info!("Capture session started, entering message loop");

    // Run Windows message loop - REQUIRED for COM events to fire
    unsafe {
        let mut msg = std::mem::zeroed();
        while !should_stop.load(Ordering::Relaxed) {
            // PeekMessage is non-blocking - check for messages
            if PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    info!("WM_QUIT received");
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            } else {
                // No messages, sleep briefly to avoid CPU spinning
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    }

    // Cleanup
    let _ = item.RemoveClosed(closed_token);
    drop(frame_pool); // Calls Drop which closes session and pool

    info!("Graphics capture stopped cleanly");
    Ok(())
}

fn on_frame_arrived<H: CaptureHandler>(should_stop: Arc<AtomicBool>, staging: Arc<StagingTexture>, mut handler: Arc<Mutex<H>>)
    -> EventCallbackFn<Direct3D11CaptureFramePool> {
    TypedEventHandler::new(
        move |pool: Ref<Direct3D11CaptureFramePool>, _| {
            if should_stop.load(Ordering::Relaxed) {
                return Ok(());
            }

            let pool = pool.as_ref().unwrap();

            // Try to get next frame
            let frame = match pool.TryGetNextFrame() {
                Ok(f) => f,
                Err(_) => return Ok(()), // No frame available yet
            };

            // Get frame surface
            let surface = frame.Surface()?;
            let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
            let texture: ID3D11Texture2D = unsafe { access.GetInterface()? };

            // Get texture description
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            unsafe { texture.GetDesc(&mut desc) };

            let device = unsafe { texture.GetDevice()? };
            let mut ctx = unsafe { device.GetImmediateContext()? };

            // Copy GPU texture to CPU-readable staging texture
            unsafe { ctx.CopyResource(staging.texture(), &texture) };

            // Map staging texture to get CPU pointer
            let mut mapped = unsafe { std::mem::zeroed() };
            unsafe {
                ctx.Map(staging.texture(), 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;
            };

            // Copy pixel data to Vec<u8>
            let data_size = (mapped.RowPitch * desc.Height) as usize;
            let mut pixel_data = vec![0u8; data_size];
            unsafe {
                std::ptr::copy_nonoverlapping(
                    mapped.pData as *const u8,
                    pixel_data.as_mut_ptr(),
                    data_size,
                );
            };

            // Unmap
            unsafe { ctx.Unmap(staging.texture(), 0) };

            // Create captured frame
            let captured = CapturedFrame {
                data: pixel_data,
                width: desc.Width,
                height: desc.Height,
                stride: mapped.RowPitch,
                format: PixelFormat::BGRA,
                timestamp: std::time::Instant::now(),
            };

            // Call user's handler
            if let Ok(mut handler_mut) = handler.lock() {
                if let Err(e) = handler_mut.on_frame_arrived(captured) {
                    error!("Handler error: {}", e);
                }
            }

            Ok(())
        }
    )
}

fn on_capture_closed<H: CaptureHandler>(should_stop: Arc<AtomicBool>, handler: Arc<Mutex<H>>) -> EventCallbackFn<GraphicsCaptureItem> {
    TypedEventHandler::new(move |_, _| {
        info!("Capture closed by system");
        should_stop.store(true, Ordering::Relaxed);
        if let Ok(mut handler_mut) = handler.lock() {
            handler_mut.on_capture_closed();
        }

        // Post quit to exit message loop
        unsafe {
            use windows::Win32::System::Threading::GetCurrentThreadId;
            let _ = PostThreadMessageW(
                GetCurrentThreadId(),
                WM_QUIT,
                WPARAM::default(),
                LPARAM::default()
            );
        }

        Ok(())
    })
}