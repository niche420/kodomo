use std::{mem, ptr};
use anyhow::Error;
use ffmpeg_next::format::Pixel;
use windows::core::Interface;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D11::{D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_FLAG, D3D11_MAP_READ, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Dxgi::{IDXGIDevice, IDXGIOutput1, IDXGIOutputDuplication};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use crate::capture::{Frame, FrameCapturer, PixelFormat};

pub struct DxgiCapturer
{
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    output_duplication: IDXGIOutputDuplication,
    staging_texture: ID3D11Texture2D,
    frame_width: u32,
    frame_height: u32
}

impl DxgiCapturer
{
    pub fn new() -> anyhow::Result<DxgiCapturer>
    {
        unsafe {
            let mut device_opt = None;
            let mut context_opt = None;
            D3D11CreateDevice(None, D3D_DRIVER_TYPE_HARDWARE, HMODULE::default(),
                              D3D11_CREATE_DEVICE_FLAG(0), None, D3D11_SDK_VERSION,
                              Some(&mut device_opt),
                              None,
                              Some(&mut context_opt))?;
            let device = device_opt.unwrap();
            let context = context_opt.unwrap();
            let dxgi_device = device.cast::<IDXGIDevice>()?;
            let adapter = dxgi_device.GetAdapter()?;
            let output = adapter.EnumOutputs(0)?;
            let output_1 = output.cast::<IDXGIOutput1>()?;
            let output_duplication = output_1.DuplicateOutput(&device)?;

            let output_desc = output_1.GetDesc()?;
            let staging_desc = D3D11_TEXTURE2D_DESC {
                Width: (output_desc.DesktopCoordinates.right - output_desc.DesktopCoordinates.left) as u32,
                Height: (output_desc.DesktopCoordinates.bottom - output_desc.DesktopCoordinates.top) as u32,
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
            device.CreateTexture2D(&staging_desc, None, Some(&mut staging_opt))?;
            let staging_texture = staging_opt.unwrap();

            Ok(Self {
                device,
                context,
                output_duplication,
                staging_texture,
                frame_width: staging_desc.Width,
                frame_height: staging_desc.Height,
            })
        }
    }
}

impl FrameCapturer for DxgiCapturer
{
    fn capture_frame(&mut self) -> anyhow::Result<Frame>
    {
        unsafe {
            let mut frame_info = mem::zeroed();
            // 0-second timeout means nonblocking
            let mut desktop_resource = None;
            self.output_duplication.AcquireNextFrame(
                0, &mut frame_info, &mut desktop_resource)?;

            // Copy gpu texture onto staging texture
            let gpu_texture = desktop_resource.unwrap().cast::<ID3D11Texture2D>()?;
            self.context.CopyResource(&self.staging_texture, &gpu_texture);

            // Map staging texture so that we can read it on the CPU
            let mut mapped = mem::zeroed();
            let map_result = self.context.Map(&self.staging_texture, 0,
                             D3D11_MAP_READ, 0, Some(&mut mapped)).map(|_| mapped);
            mapped = match map_result {
                Ok(mapped) => mapped,
                Err(err) => {
                    let _ = self.output_duplication.ReleaseFrame();
                    return Err(Error::from(err));
                }
            };

            // Copy pixel data
            let row_pitch = mapped.RowPitch as usize;
            let data_size = row_pitch * self.frame_height as usize;
            let mut data = vec![0u8; data_size];

            ptr::copy_nonoverlapping(
                mapped.pData as *const u8,
                data.as_mut_ptr(),
                data_size,
            );

            // Unmap
            self.context.Unmap(&self.staging_texture, 0);

            // Release frame
            self.output_duplication.ReleaseFrame()?;

            Ok(Frame {
                format: PixelFormat::Bgra,
                width: self.frame_width,
                height: self.frame_height,
                data
            })
        }
    }
}