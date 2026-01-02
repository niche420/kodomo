mod d3d11;
mod window;
mod frame_pool;
mod monitor;
mod dd;
mod gfx_capture;

use std::{mem, ptr};
use std::time::Instant;
use tracing::info;
use windows::{
    core::*,
    Win32::Graphics::{
        Direct3D11::*,
        Dxgi::{Common::*, *},
    },
};
use windows::Graphics::DirectX::DirectXPixelFormat;
use crate::{CaptureConfig, CaptureError, CaptureMode, CapturedFrame, MonitorInfo, PixelFormat, ScreenCapture};
use crate::win32::dd::DxgiDuplication;
use crate::win32::gfx_capture::GfxCapture;
use crate::win32::monitor::Monitor;
use crate::win32::window::Window;

#[derive(Debug)]
pub enum CaptureItemType {
    Monitor(DxgiDuplication),
    Window(GfxCapture),
    Unknown,
}

#[derive(Debug, Default)]
pub struct DirectXCapture(Option<CaptureItemType>);

impl ScreenCapture for DirectXCapture {
    fn init(&mut self, config: CaptureConfig) -> crate::Result<()> {
        info!("Initializing Windows DXGI capture: {:?}", config);

        self.0 = match config.mode {
            CaptureMode::Window(name) => {
                Some(CaptureItemType::Window(GfxCapture::new(name)?))
            }
            CaptureMode::Monitor(idx) => {
                Some(CaptureItemType::Monitor(DxgiDuplication::from_idx(idx)?))
            }
            CaptureMode::Unknown => {
                None
            }
        };

        Ok(())
    }

    fn capture_frame(&mut self) -> crate::Result<CapturedFrame> {

    }

    fn get_monitors(&self) -> crate::Result<Vec<MonitorInfo>> {
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

    fn shutdown(&mut self) -> crate::Result<()> {
        info!("Shutting down Windows capture");
        Ok(())
    }
}