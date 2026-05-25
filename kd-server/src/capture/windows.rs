use std::{mem, ptr};
use std::sync::{Arc, Mutex};
use anyhow::Error;
use ffmpeg_next::format::Pixel;
use windows::core::{Interface, Ref};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D11::{D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_FLAG, D3D11_MAP_READ, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Dxgi::{IDXGIDevice, IDXGIOutput1, IDXGIOutputDuplication};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Gdi::MonitorFromWindow;
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::System::WinRT::Direct3D11::{CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess};
use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;
use crate::capture::{Frame, FrameCapturer, PixelFormat};

pub struct WindowsGraphicsCapturer {
    frame_pool: Direct3D11CaptureFramePool,
    session: GraphicsCaptureSession,
    device: ID3D11Device,
    staging: Arc<Mutex<ID3D11Texture2D>>,
    latest_frame: Arc<Mutex<Option<Vec<u8>>>>,
    width: u32,
    height: u32,
}

impl WindowsGraphicsCapturer {
    pub fn new() -> anyhow::Result<WindowsGraphicsCapturer> {
        unsafe {
            let mut device = None;
            let mut ctx = None;
            let feature_level = D3D_FEATURE_LEVEL_11_0;

            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_FLAG(0),
                Some(&[feature_level]),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut ctx),
            )?;

            let monitor = MonitorFromWindow(GetDesktopWindow(), Default::default());
            let interop: IGraphicsCaptureItemInterop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
            let item: GraphicsCaptureItem = interop.CreateForMonitor(monitor)?;

            let dxgi_device = device.as_ref().unwrap().cast::<IDXGIDevice>()?;
            let winrt_device: IDirect3DDevice = CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)?.cast()?;
            let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
                &winrt_device,
                DirectXPixelFormat::B8G8R8A8UIntNormalized,
                2,
                item.Size()?,
            )?;

            let latest_frame = Arc::new(Mutex::new(None::<Vec<u8>>));
            let latest_frame_clone = latest_frame.clone();

            let size = item.Size()?;
            let width = size.Width as u32;
            let height = size.Height as u32;
            let staging_desc = D3D11_TEXTURE2D_DESC {
                Width: width,
                Height: height,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_STAGING,
                BindFlags: 0u32,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                MiscFlags: 0u32,
            };
            let mut staging_opt = None;
            device.as_ref().unwrap().CreateTexture2D(&staging_desc, None, Some(&mut staging_opt))?;

            let staging = Arc::new(Mutex::new(staging_opt.unwrap()));
            let staging_texture_clone = staging.clone();
            frame_pool.FrameArrived(&TypedEventHandler::new(move |pool: Ref<Direct3D11CaptureFramePool>, _| {
                let pool = pool.as_ref().unwrap();
                let frame = pool.TryGetNextFrame()?;
                let surface = frame.Surface()?;
                let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
                let texture: ID3D11Texture2D = access.GetInterface()?;

                // Copy GPU texture to staging texture
                let staging = staging_texture_clone.lock().unwrap();
                let device = texture.GetDevice()?;
                let context = device.GetImmediateContext()?;
                context.CopyResource(&*staging, &texture);

                // Map the staging texture
                let mut mapped = std::mem::zeroed();
                context.Map(&*staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;

                // Copy pixel data
                let row_pitch = mapped.RowPitch as usize;
                let data_size = row_pitch * height as usize;
                let mut data = vec![0u8; data_size];
                std::ptr::copy_nonoverlapping(mapped.pData as *const u8, data.as_mut_ptr(), data_size);

                // Unmap
                context.Unmap(&*staging, 0);

                // Store
                *latest_frame_clone.lock().unwrap() = Some(data);
                Ok(())
            }))?;

            let session = frame_pool.CreateCaptureSession(&item)?;
            session.StartCapture()?;

            Ok(Self {
                device: device.unwrap(),
                frame_pool,
                session,
                latest_frame,
                width,
                height,
                staging,
            })
        }
    }
}

impl FrameCapturer for WindowsGraphicsCapturer {
    fn capture_frame(&mut self) -> Option<Frame> {
        let data = self.latest_frame.lock().unwrap().take();
        match data {
            Some(data) => Some(Frame {
                format: PixelFormat::Bgra,
                width: self.width,
                height: self.height,
                data,
            }),
            None => None
        }
    }
}
