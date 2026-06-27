use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail};
use windows::core::{Interface, BOOL};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{
    Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::{HMODULE, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11Texture2D, D3D11_CPU_ACCESS_READ,
    D3D11_CREATE_DEVICE_FLAG, D3D11_MAP_READ, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_SAMPLE_DESC, DXGI_FORMAT_B8G8R8A8_UNORM};
use windows::Win32::Graphics::Dxgi::{IDXGIDevice};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::System::WinRT::Direct3D11::{
    CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowLongW, GetWindowRect, GetWindowThreadProcessId, IsWindowVisible,
    GWL_STYLE, WS_VISIBLE,
};

use crate::capture::{Frame, FrameCapturer, PixelFormat};

// ─── Window finding ───────────────────────────────────────────────────────────

struct EnumState {
    target_name: String, // lowercased exe filename only
    candidates: Vec<(HWND, u64)>, // (hwnd, area)
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let state = &mut *(lparam.0 as *mut EnumState);

    if !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1);
    }

    let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
    if style & WS_VISIBLE.0 == 0 {
        return BOOL(1);
    }

    let mut rect = RECT::default();
    if GetWindowRect(hwnd, &mut rect).is_err() {
        return BOOL(1);
    }
    let w = (rect.right - rect.left) as i64;
    let h = (rect.bottom - rect.top) as i64;
    if w < 400 || h < 300 {
        return BOOL(1);
    }

    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 {
        return BOOL(1);
    }

    let proc = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
        Ok(h) => h,
        Err(_) => return BOOL(1),
    };

    let mut buf = vec![0u16; 1024];
    let mut len = buf.len() as u32;
    let ok = QueryFullProcessImageNameW(
        proc,
        PROCESS_NAME_WIN32,
        windows::core::PWSTR(buf.as_mut_ptr()),
        &mut len,
    );
    let _ = windows::Win32::Foundation::CloseHandle(proc);

    if ok.is_err() {
        return BOOL(1);
    }

    let path = String::from_utf16_lossy(&buf[..len as usize]);
    let found_name = PathBuf::from(&path)
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    if found_name == state.target_name {
        state.candidates.push((hwnd, (w * h) as u64));
    }

    BOOL(1)
}

fn find_game_window(exe_path: &PathBuf) -> Option<HWND> {
    let target_name = exe_path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let mut state = EnumState { target_name, candidates: Vec::new() };
    unsafe {
        let _ = EnumWindows(
            Some(enum_windows_callback),
            LPARAM(&mut state as *mut EnumState as isize),
        );
    }
    state.candidates.into_iter().max_by_key(|c| c.1).map(|c| c.0)
}

fn find_or_launch_window(exe_path: &PathBuf, timeout: Duration) -> anyhow::Result<HWND> {
    if let Some(hwnd) = find_game_window(exe_path) {
        eprintln!("capture: found existing window for {:?}", exe_path.file_name().unwrap_or_default());
        return Ok(hwnd);
    }

    eprintln!("capture: launching {:?}", exe_path);
    std::process::Command::new(exe_path)
        .spawn()
        .map_err(|e| anyhow!("Failed to launch game: {e}"))?;

    let start = Instant::now();
    loop {
        std::thread::sleep(Duration::from_millis(100));
        if let Some(hwnd) = find_game_window(exe_path) {
            eprintln!("capture: window appeared after {:.1}s", start.elapsed().as_secs_f32());
            return Ok(hwnd);
        }
        if start.elapsed() >= timeout {
            bail!("Timed out waiting for game window after {}s", timeout.as_secs());
        }
    }
}

// ─── Capturer ─────────────────────────────────────────────────────────────────

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
    pub fn new(exe_path: &PathBuf) -> anyhow::Result<Self> {
        let hwnd = find_or_launch_window(exe_path, Duration::from_secs(30))?;
        unsafe { Self::new_for_window(hwnd) }
    }

    unsafe fn new_for_window(hwnd: HWND) -> anyhow::Result<Self> {
        let mut device = None;
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_FLAG(0),
            Some(&[D3D_FEATURE_LEVEL_11_0]),
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            None,
        )?;
        let device = device.unwrap();

        let interop: IGraphicsCaptureItemInterop =
            windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        let item: GraphicsCaptureItem = interop.CreateForWindow(hwnd)?;

        let dxgi_device = device.cast::<IDXGIDevice>()?;
        let winrt_device: IDirect3DDevice =
            CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)?.cast()?;

        let size = item.Size()?;
        let width  = size.Width  as u32;
        let height = size.Height as u32;

        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &winrt_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            size,
        )?;

        let staging_desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0u32,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            MiscFlags: 0u32,
        };
        let mut staging_opt = None;
        device.CreateTexture2D(&staging_desc, None, Some(&mut staging_opt))?;
        let staging = Arc::new(Mutex::new(staging_opt.unwrap()));
        let staging_clone = staging.clone();

        let latest_frame: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
        let latest_frame_clone = latest_frame.clone();

        frame_pool.FrameArrived(&TypedEventHandler::new(
            move |pool: windows::core::Ref<Direct3D11CaptureFramePool>, _| {
                let pool = pool.as_ref().unwrap();
                let frame = pool.TryGetNextFrame()?;
                let surface = frame.Surface()?;
                let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
                let texture: ID3D11Texture2D = access.GetInterface()?;

                let staging = staging_clone.lock().unwrap();
                let device  = texture.GetDevice()?;
                let context = device.GetImmediateContext()?;
                context.CopyResource(&*staging, &texture);

                let mut mapped = std::mem::zeroed();
                context.Map(&*staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;

                let row_pitch = mapped.RowPitch as usize;
                let mut data = vec![0u8; row_pitch * height as usize];
                std::ptr::copy_nonoverlapping(
                    mapped.pData as *const u8,
                    data.as_mut_ptr(),
                    data.len(),
                );
                context.Unmap(&*staging, 0);

                *latest_frame_clone.lock().unwrap() = Some(data);
                Ok(())
            },
        ))?;

        let session = frame_pool.CreateCaptureSession(&item)?;
        session.StartCapture()?;

        Ok(Self { device, frame_pool, session, latest_frame, staging, width, height })
    }
}

impl FrameCapturer for WindowsGraphicsCapturer {
    fn capture_frame(&mut self) -> Option<Frame> {
        let data = self.latest_frame.lock().unwrap().take()?;
        Some(Frame {
            format: PixelFormat::Bgra,
            width: self.width,
            height: self.height,
            data,
        })
    }
}