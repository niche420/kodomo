use crate::{PixelFormat, Result};
use std::mem;
use std::ptr;
use std::sync::{atomic, Arc, Mutex};
use std::sync::atomic::AtomicBool;
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
use windows::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};
use crate::{CaptureConfig, CaptureError, CapturedFrame, MonitorInfo, ScreenCapture};
use crate::win32::{d3d11, frame_pool, window};
use crate::win32::frame_pool::{FramePool};
use crate::win32::window::Window;

pub type EventCallbackToken = i64;
pub type EventCallbackFn<Sender> = TypedEventHandler::<Sender, IInspectable>;

#[derive(Debug)]
pub struct GfxCapture {
    device: d3d11::Device,
    frame_pool: FramePool,
    item: GraphicsCaptureItem,
    capture_closed: EventCallbackToken
}

impl GfxCapture {
    pub fn new(wnd_name: String) -> Result<Self> {
        info!("Initializing Windows D3D11 Graphics Capture");

        let device = d3d11::Device::new()?;
        let wnd = Window::find_by_name(&*wnd_name)?;

        let item: GraphicsCaptureItem = wnd.try_into()?;
        let mut frame_pool = FramePool::new(&device, DirectXPixelFormat::B8G8R8A8UIntNormalized, &item)?;

        let halt = Arc::new(AtomicBool::new(false));
        let capture_closed =
            item.Closed(&on_capture_closed())?;
        frame_pool.set_frame_arrived(on_frame_arrived())?;

        Ok(Self {
            device,
            frame_pool,
            item,
            capture_closed
        })
    }
}

fn on_capture_closed() -> EventCallbackFn<GraphicsCaptureItem> {
    EventCallbackFn::new(move |_, _| {
        // Send quit msg
        unsafe {
            PostThreadMessageW(GetCurrentThreadId(), WM_QUIT, WPARAM::default(), LPARAM::default())?;
        };

        Ok(())
    })
}

fn on_frame_arrived(halt: Arc<AtomicBool>, pixel_fmt: PixelFormat) -> EventCallbackFn<Direct3D11CaptureFramePool> {
    // Init
    let frame_pool_recreate = frame_pool.clone();
    let d3d_device_frame_pool = d3d_device.clone();
    let context = d3d_device_context.clone();
    let result_frame_pool = result;

    let last_size = item.Size()?;
    let last_size = Arc::new((AtomicI32::new(last_size.Width), AtomicI32::new(last_size.Height)));
    let callback_frame_pool = callback;
    let direct3d_device_recreate = SendDirectX::new(direct3d_device.clone());

    EventCallbackFn::new(move |frame: Ref<Direct3D11CaptureFramePool>, _| {
        // Return early if the capture is closed
        if halt.load(atomic::Ordering::Relaxed) {
            return Ok(());
        }

        // Get frame
        let frame: Direct3D11CaptureFrame = frame.as_ref()
            .expect("FramePool given by FrameArrived callback was None")
            .TryGetNextFrame()?;

        // Get frame content size
        let frame_content_size = frame.ContentSize()?;

        // Get frame surface
        let frame_surface = frame.Surface()?;

        // Convert surface to texture
        let frame_dxgi_interface = frame_surface.cast::<IDirect3DDxgiInterfaceAccess>()?;
        let frame_texture = unsafe { frame_dxgi_interface.GetInterface::<ID3D11Texture2D>()? };

        // Get texture settings
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { frame_texture.GetDesc(&mut desc) }

        // Check if the size has been changed
        if frame_content_size.Width != last_size.0.load(atomic::Ordering::Relaxed)
            || frame_content_size.Height != last_size.1.load(atomic::Ordering::Relaxed)
        {
            let direct3d_device_recreate = &direct3d_device_recreate;
            frame_pool_recreate.Recreate(&direct3d_device_recreate.0, pixel_fmt, 1, frame_content_size)?;

            last_size.0.store(frame_content_size.Width, atomic::Ordering::Relaxed);
            last_size.1.store(frame_content_size.Height, atomic::Ordering::Relaxed);
        }

        // Create a frame
        let mut frame = CapturedFrame::new(
            frame,
            &d3d_device_frame_pool,
            frame_surface,
            frame_texture,
            &context,
            desc,
            color_format,
            title_bar_height,
        );

        // Init internal capture control
        let stop = Arc::new(AtomicBool::new(false));

        // Send the frame to the callback struct
        let result = callback_frame_pool.lock().on_frame_arrived(&mut frame, internal_capture_control);

        // If the user signals to stop or an error occurs, halt the capture.
        if stop.load(atomic::Ordering::Relaxed) || result.is_err() {
            if let Err(e) = result {
                *result_frame_pool.lock() = Some(e);
            }

            halt.store(true, atomic::Ordering::Relaxed);

            // Stop the message loop to allow the thread to exit gracefully.
            unsafe {
                PostThreadMessageW(GetCurrentThreadId(), WM_QUIT, WPARAM::default(), LPARAM::default())?;
            };
        }

        Ok(())
    })
}

impl Drop for GfxCapture {
    fn drop(&mut self) {
        let _ = self.item.RemoveClosed(self.capture_closed);
    }
}