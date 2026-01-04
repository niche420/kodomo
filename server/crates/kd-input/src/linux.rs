#[cfg(target_os = "linux")]
pub mod linux {
    use super::*;
    use std::ptr;
    use tracing::{debug, info};
    use x11::xlib::*;
    use x11::xtest::*;

    pub struct LinuxInputInjector {
        display: Option<*mut Display>,
        config: Option<InputConfig>,
    }

    impl LinuxInputInjector {
        pub fn new() -> Result<Self> {
            Ok(Self {
                display: None,
                config: None,
            })
        }

        fn keycode_to_x11(&self, key: KeyCode) -> u32 {
            // X11 keycodes are offset by 8 from Windows VK codes
            // This is a simplified mapping
            (key as u32).wrapping_sub(8)
        }
    }

    impl InputInjector for LinuxInputInjector {
        fn init(&mut self, config: InputConfig) -> Result<()> {
            info!("Initializing Linux input injection");

            unsafe {
                let display = XOpenDisplay(ptr::null());
                if display.is_null() {
                    return Err(InputError::InitFailed("Cannot open X display".into()));
                }

                // Check if XTest extension is available
                let mut event_base = 0;
                let mut error_base = 0;
                let mut major = 0;
                let mut minor = 0;

                if XTestQueryExtension(display, &mut event_base, &mut error_base, &mut major, &mut minor) == 0 {
                    XCloseDisplay(display);
                    return Err(InputError::InitFailed("XTest extension not available".into()));
                }

                self.display = Some(display);
                self.config = Some(config);

                info!("XTest extension available: v{}.{}", major, minor);
                Ok(())
            }
        }

        fn inject_keyboard(&mut self, event: KeyboardEvent) -> Result<()> {
            unsafe {
                let display = self.display
                    .ok_or(InputError::InjectionFailed("Display not initialized".into()))?;

                let keycode = self.keycode_to_x11(event.key);
                let is_press = event.state == KeyState::Pressed;

                XTestFakeKeyEvent(display, keycode, if is_press { 1 } else { 0 }, 0);
                XFlush(display);

                debug!("Injected keyboard: {:?} {:?}", event.key, event.state);
                Ok(())
            }
        }

        fn inject_mouse(&mut self, event: MouseEvent) -> Result<()> {
            unsafe {
                let display = self.display
                    .ok_or(InputError::InjectionFailed("Display not initialized".into()))?;

                match event {
                    MouseEvent::Move { x, y, relative } => {
                        if relative {
                            XTestFakeRelativeMotionEvent(display, x, y, 0);
                        } else {
                            XTestFakeMotionEvent(display, -1, x, y, 0);
                        }
                    }
                    MouseEvent::Button { button, state } => {
                        let button_code = match button {
                            MouseButton::Left => 1,
                            MouseButton::Middle => 2,
                            MouseButton::Right => 3,
                            MouseButton::X1 => 8,
                            MouseButton::X2 => 9,
                        };

                        let is_press = state == ButtonState::Pressed;
                        XTestFakeButtonEvent(display, button_code, if is_press { 1 } else { 0 }, 0);
                    }
                    MouseEvent::Wheel { delta_x: _, delta_y } => {
                        let button = if delta_y > 0 { 4 } else { 5 }; // Scroll up / down
                        XTestFakeButtonEvent(display, button, 1, 0);
                        XTestFakeButtonEvent(display, button, 0, 0);
                    }
                }

                XFlush(display);
                debug!("Injected mouse event");
                Ok(())
            }
        }

        fn inject_gamepad(&mut self, event: GamepadEvent) -> Result<()> {
            // TODO: Implement uinput virtual gamepad
            debug!("Gamepad injection not yet implemented on Linux");
            Ok(())
        }

        fn shutdown(&mut self) -> Result<()> {
            info!("Shutting down Linux input injection");

            if let Some(display) = self.display {
                unsafe {
                    XCloseDisplay(display);
                }
            }

            self.display = None;
            Ok(())
        }
    }
}