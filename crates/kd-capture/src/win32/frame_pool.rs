use std::sync::{atomic, Arc};
use windows::core::IInspectable;
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::Graphics::Direct3D11::{ID3D11Texture2D, D3D11_TEXTURE2D_DESC};
use windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess;
use windows::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};
use crate::win32::window::Window;
use crate::Result;
use crate::win32::d3d11::Device;
use crate::win32::gfx_capture::{EventCallbackFn, EventCallbackToken};

#[derive(Debug)]
pub(crate) struct FramePool {
    pool: Arc<Direct3D11CaptureFramePool>,
    session: GraphicsCaptureSession,
    frame_arrived: Option<EventCallbackToken>
}

impl FramePool {
    pub fn new(device: &Device, pixel_fmt: DirectXPixelFormat, item: &GraphicsCaptureItem) -> Result<Self> {
        let frame_pool = Direct3D11CaptureFramePool::Create(
            &device.to_d3d()?, pixel_fmt, 1, item.Size()?)?;
        let frame_pool = Arc::new(frame_pool);
        let session = frame_pool.CreateCaptureSession(item)?;

        Ok(Self {
            pool: frame_pool,
            session,
            frame_arrived: None
        })
    }

    pub fn set_frame_arrived(&mut self, frame_arrived: EventCallbackFn<Direct3D11CaptureFramePool>) -> Result<()> {
        self.frame_arrived = Some(self.pool.FrameArrived(&frame_arrived)?);
        Ok(())
    }
    
    pub fn start(&mut self) -> Result<()> {
        self.session.StartCapture()?;
        Ok(())
    }
}

impl Drop for FramePool {
    fn drop(&mut self) {
        if let Some(frame_arrived) = self.frame_arrived {
            let _ = self.pool.RemoveFrameArrived(frame_arrived);
        }
        let _ = self.pool.Close();
        let _ = self.session.Close();
    }
}