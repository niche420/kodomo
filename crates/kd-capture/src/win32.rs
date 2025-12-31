#[cfg(target_os = "windows")]
pub mod kd_win32 {
    use crate::{PixelFormat, Result};
    use std::mem;
    use std::ptr;
    use std::time::Instant;
    use tracing::{debug, error, info};
    use windows::{
        core::*,
        Win32::Graphics::{
            Direct3D11::*,
            Dxgi::{Common::*, *},
        },
    };
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
    use crate::{CaptureConfig, CaptureError, CapturedFrame, MonitorInfo, ScreenCapture};

    pub struct DxgiCapture {
        device: Option<ID3D11Device>,
        context: Option<ID3D11DeviceContext>,
        output_duplication: Option<IDXGIOutputDuplication>,
        staging_texture: Option<ID3D11Texture2D>,
        config: Option<CaptureConfig>,
    }

    impl DxgiCapture {
        pub fn new() -> Result<Self> {
            Ok(Self {
                device: None,
                context: None,
                output_duplication: None,
                staging_texture: None,
                config: None,
            })
        }

        fn init_d3d11(&mut self) -> Result<()> {
            unsafe {
                let mut device = None;
                let mut context = None;
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
                    Some(&mut context),
                )
                    .map_err(|e| CaptureError::InitFailed(format!("D3D11 device: {}", e)))?;

                self.device = device;
                self.context = context;

                info!("D3D11 device created successfully");
                Ok(())
            }
        }

        fn init_output_duplication(&mut self, monitor_index: u32) -> Result<()> {
            unsafe {
                let device = self.device.as_ref()
                    .ok_or(CaptureError::InitFailed("Device not initialized".into()))?;

                // Get DXGI device from D3D11 device
                let dxgi_device: IDXGIDevice = device.cast()
                    .map_err(|e| CaptureError::InitFailed(format!("Cast to IDXGIDevice: {}", e)))?;

                // Get adapter
                let adapter = dxgi_device.GetAdapter()
                    .map_err(|e| CaptureError::InitFailed(format!("GetAdapter: {}", e)))?;

                // Get output (monitor)
                let output = adapter.EnumOutputs(monitor_index)
                    .map_err(|e| CaptureError::InitFailed(
                        format!("Monitor {} not found: {}", monitor_index, e)
                    ))?;

                let output1: IDXGIOutput1 = output.cast()
                    .map_err(|e| CaptureError::InitFailed(format!("Cast to IDXGIOutput1: {}", e)))?;

                // Create desktop duplication
                let duplication = output1.DuplicateOutput(device)
                    .map_err(|e| CaptureError::InitFailed(format!("DuplicateOutput: {}", e)))?;

                self.output_duplication = Some(duplication);

                info!("DXGI output duplication initialized for monitor {}", monitor_index);
                Ok(())
            }
        }

        fn create_staging_texture(&mut self, width: u32, height: u32) -> Result<()> {
            unsafe {
                let device = self.device.as_ref()
                    .ok_or(CaptureError::InitFailed("Device not initialized".into()))?;

                let desc = D3D11_TEXTURE2D_DESC {
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

                let mut texture: Option<ID3D11Texture2D> = None;
                device.CreateTexture2D(&desc, None, Some(&mut texture))
                    .map_err(|e| CaptureError::InitFailed(format!("Staging texture: {}", e)))?;

                self.staging_texture = texture;

                debug!("Staging texture created: {}x{}", width, height);
                Ok(())
            }
        }
    }

    impl ScreenCapture for DxgiCapture {
        fn init(&mut self, config: CaptureConfig) -> Result<()> {
            info!("Initializing Windows DXGI capture: {:?}", config);

            self.init_d3d11()?;
            self.init_output_duplication(config.monitor_index)?;
            self.create_staging_texture(config.width, config.height)?;

            self.config = Some(config);
            Ok(())
        }

        fn capture_frame(&mut self) -> Result<CapturedFrame> {
            unsafe {
                let duplication = self.output_duplication.as_ref()
                    .ok_or(CaptureError::CaptureFailed("Not initialized".into()))?;

                let context = self.context.as_ref()
                    .ok_or(CaptureError::CaptureFailed("Context not initialized".into()))?;

                let staging = self.staging_texture.as_ref()
                    .ok_or(CaptureError::CaptureFailed("Staging texture not initialized".into()))?;

                // Acquire next frame (0ms timeout = non-blocking)
                let mut frame_info = mem::zeroed();
                let mut desktop_resource = None;

                let result = duplication.AcquireNextFrame(0, &mut frame_info, &mut desktop_resource);

                if result.is_err() {
                    // DXGI_ERROR_WAIT_TIMEOUT means no new frame yet
                    return Err(CaptureError::NoFrame);
                }

                let desktop_resource = desktop_resource
                    .ok_or(CaptureError::CaptureFailed("No desktop resource".into()))?;

                // Cast to texture
                let texture: ID3D11Texture2D = desktop_resource.cast()
                    .map_err(|e| CaptureError::CaptureFailed(format!("Cast texture: {}", e)))?;

                // Copy to staging texture (GPU -> CPU accessible memory)
                context.CopyResource(staging, &texture);

                // Map staging texture to CPU memory
                let mut mapped = mem::zeroed();
                context.Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                    .map_err(|e| CaptureError::CaptureFailed(format!("Map failed: {}", e)))?;

                // Copy pixel data
                let config = self.config.as_ref().unwrap();
                let row_pitch = mapped.RowPitch as usize;
                let data_size = row_pitch * config.height as usize;
                let mut data = vec![0u8; data_size];

                ptr::copy_nonoverlapping(
                    mapped.pData as *const u8,
                    data.as_mut_ptr(),
                    data_size,
                );

                // Unmap
                context.Unmap(staging, 0);

                // Release frame
                let _ = duplication.ReleaseFrame();

                Ok(CapturedFrame {
                    data,
                    width: config.width,
                    height: config.height,
                    stride: row_pitch as u32,
                    format: PixelFormat::BGRA,
                    timestamp: Instant::now(),
                })
            }
        }

        fn get_monitors(&self) -> Result<Vec<MonitorInfo>> {
            // TODO: Enumerate all monitors
            Ok(vec![MonitorInfo {
                id: 0,
                name: "Primary Monitor".into(),
                width: 1920,
                height: 1080,
                refresh_rate: 60,
                is_primary: true,
            }])
        }

        fn shutdown(&mut self) -> Result<()> {
            info!("Shutting down Windows capture");
            self.output_duplication = None;
            self.staging_texture = None;
            self.context = None;
            self.device = None;
            Ok(())
        }
    }
}