use tracing::info;
use windows::core::Interface;
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D11::{D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_CPU_ACCESS_READ, D3D11_CPU_ACCESS_WRITE, D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice;
use crate::{CaptureError, Result };

#[derive(Debug)]
pub struct Device {
    device: Option<ID3D11Device>,
    ctx: Option<ID3D11DeviceContext>,
}

impl Device {
    pub fn new() -> Result<Self> {
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
            )
                .map_err(|e| CaptureError::InitFailed(format!("D3D11 device: {}", e)))?;

            info!("D3D11 device created successfully");
            Ok(Self {
                device,
                ctx
            })
        }
    }

    pub fn to_d3d(&self) -> Result<IDirect3DDevice> {
        if let Some(device) = &self.device {
            let dxgi_device: IDXGIDevice = device.cast()?;
            let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)? };
            let dev: IDirect3DDevice = inspectable.cast()?;

            return Ok(dev)
        }

        Err(CaptureError::InitFailed(format!("Device not initialized")))
    }

    pub fn device(&self) -> &ID3D11Device {
        self.device.as_ref().unwrap()
    }
}

pub struct StagingTexture {
    inner: ID3D11Texture2D,
    desc: D3D11_TEXTURE2D_DESC,
    is_mapped: bool,
}

impl StagingTexture {
    /// Create a staging texture suitable for CPU read/write with the given geometry/format.
    pub fn new(device: &ID3D11Device, width: u32, height: u32, format: DXGI_FORMAT) -> Result<Self> {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 1,
            ArraySize: 1,
            Format: format,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: (D3D11_CPU_ACCESS_READ.0 | D3D11_CPU_ACCESS_WRITE.0) as u32,
            MiscFlags: 0,
        };

        let mut tex = None;
        unsafe {
            device.CreateTexture2D(&desc, None, Some(&mut tex))?;
        }
        Ok(Self { inner: tex.unwrap(), desc, is_mapped: false })
    }

    /// Gets the underlying [`windows::Win32::Graphics::Direct3D11::ID3D11Texture2D`].
    #[inline]
    #[must_use]
    pub const fn texture(&self) -> &ID3D11Texture2D {
        &self.inner
    }

    /// Gets the description of the texture.
    #[inline]
    #[must_use]
    pub const fn desc(&self) -> D3D11_TEXTURE2D_DESC {
        self.desc
    }

    /// Checks if the texture is currently mapped.
    #[inline]
    #[must_use]
    pub const fn is_mapped(&self) -> bool {
        self.is_mapped
    }

    /// Marks the texture as mapped or unmapped.
    #[inline]
    pub const fn set_mapped(&mut self, mapped: bool) {
        self.is_mapped = mapped;
    }

    /// Validate an externally constructed texture as a CPU staging texture.
    /// The texture must have been created with `D3D11_USAGE_STAGING` usage and
    /// `D3D11_CPU_ACCESS_READ` and `D3D11_CPU_ACCESS_WRITE` CPU access flags.
    pub fn from_raw_checked(tex: ID3D11Texture2D) -> Option<Self> {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { tex.GetDesc(&mut desc) };
        let is_staging = desc.Usage == D3D11_USAGE_STAGING;
        let has_cpu_rw = (desc.CPUAccessFlags & (D3D11_CPU_ACCESS_READ.0 | D3D11_CPU_ACCESS_WRITE.0) as u32) != 0;

        if !is_staging || !has_cpu_rw {
            return None;
        }

        Some(Self { inner: tex, desc, is_mapped: false })
    }
}