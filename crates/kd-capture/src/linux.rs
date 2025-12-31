#[cfg(target_os = "linux")]
pub mod linux {
    use super::*;
    use std::ptr;
    use tracing::{debug, info};
    use x11::xlib::*;
    use x11::xshm::*;

    pub struct X11Capture {
        display: Option<*mut Display>,
        window: u64,
        use_shm: bool,
        shm_info: Option<XShmSegmentInfo>,
        image: Option<*mut XImage>,
        config: Option<CaptureConfig>,
    }

    impl X11Capture {
        pub fn new() -> Result<Self> {
            Ok(Self {
                display: None,
                window: 0,
                use_shm: true,
                shm_info: None,
                image: None,
                config: None,
            })
        }

        fn init_x11(&mut self) -> Result<()> {
            unsafe {
                let display = XOpenDisplay(ptr::null());
                if display.is_null() {
                    return Err(CaptureError::InitFailed("Cannot open X display".into()));
                }

                self.display = Some(display);
                self.window = XDefaultRootWindow(display);

                // Check if MIT-SHM extension is available
                let mut major = 0;
                let mut minor = 0;
                let mut pixmaps = 0;

                self.use_shm = XShmQueryVersion(display, &mut major, &mut minor, &mut pixmaps) != 0;

                info!("X11 initialized, SHM available: {}", self.use_shm);
                Ok(())
            }
        }

        fn create_shm_image(&mut self, width: u32, height: u32) -> Result<()> {
            unsafe {
                let display = self.display.unwrap();
                let screen = XDefaultScreen(display);
                let depth = XDefaultDepth(display, screen);

                // Create XImage
                let image = XShmCreateImage(
                    display,
                    XDefaultVisual(display, screen),
                    depth as u32,
                    ZPixmap as i32,
                    ptr::null_mut(),
                    &mut self.shm_info.as_mut().unwrap() as *mut _ as *mut XShmSegmentInfo,
                    width,
                    height,
                );

                if image.is_null() {
                    return Err(CaptureError::InitFailed("Failed to create XImage".into()));
                }

                self.image = Some(image);

                // Allocate shared memory
                let size = (*image).bytes_per_line * (*image).height;
                let shmid = libc::shmget(
                    libc::IPC_PRIVATE,
                    size as usize,
                    libc::IPC_CREAT | 0o777,
                );

                if shmid < 0 {
                    return Err(CaptureError::InitFailed("shmget failed".into()));
                }

                let shmaddr = libc::shmat(shmid, ptr::null(), 0);
                if shmaddr as isize == -1 {
                    return Err(CaptureError::InitFailed("shmat failed".into()));
                }

                let mut shm_info = self.shm_info.as_mut().unwrap();
                shm_info.shmid = shmid;
                shm_info.shmaddr = shmaddr as *mut i8;
                shm_info.readOnly = 0;

                (*image).data = shmaddr as *mut i8;

                // Attach to X server
                XShmAttach(display, shm_info);

                debug!("SHM image created: {}x{}", width, height);
                Ok(())
            }
        }
    }

    impl ScreenCapture for X11Capture {
        fn init(&mut self, config: CaptureConfig) -> Result<()> {
            info!("Initializing Linux X11 capture: {:?}", config);

            self.init_x11()?;

            if self.use_shm {
                self.shm_info = Some(unsafe { std::mem::zeroed() });
                self.create_shm_image(config.width, config.height)?;
            }

            self.config = Some(config);
            Ok(())
        }

        fn capture_frame(&mut self) -> Result<CapturedFrame> {
            unsafe {
                let display = self.display.unwrap();
                let image = self.image.ok_or(CaptureError::CaptureFailed("No image".into()))?;
                let config = self.config.as_ref().unwrap();

                // Capture using SHM
                if self.use_shm {
                    XShmGetImage(
                        display,
                        self.window,
                        image,
                        0, 0,
                        0xFFFFFFFF,
                    );
                } else {
                    // Fallback to XGetImage (slower)
                    return Err(CaptureError::CaptureFailed("Non-SHM not implemented".into()));
                }

                // Copy image data
                let bytes_per_line = (*image).bytes_per_line as usize;
                let height = (*image).height as usize;
                let data_size = bytes_per_line * height;

                let mut data = vec![0u8; data_size];
                ptr::copy_nonoverlapping(
                    (*image).data as *const u8,
                    data.as_mut_ptr(),
                    data_size,
                );

                Ok(CapturedFrame {
                    data,
                    width: config.width,
                    height: config.height,
                    stride: bytes_per_line as u32,
                    format: PixelFormat::BGRA,
                    timestamp: Instant::now(),
                })
            }
        }

        fn get_monitors(&self) -> Result<Vec<MonitorInfo>> {
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
            info!("Shutting down Linux capture");

            unsafe {
                if let Some(display) = self.display {
                    if let Some(image) = self.image {
                        XDestroyImage(image);
                    }
                    if let Some(shm_info) = &self.shm_info {
                        XShmDetach(display, shm_info as *const _ as *mut XShmSegmentInfo);
                        libc::shmdt(shm_info.shmaddr as *const libc::c_void);
                        libc::shmctl(shm_info.shmid, libc::IPC_RMID, ptr::null_mut());
                    }
                    XCloseDisplay(display);
                }
            }

            self.display = None;
            self.image = None;
            self.shm_info = None;
            Ok(())
        }
    }
}