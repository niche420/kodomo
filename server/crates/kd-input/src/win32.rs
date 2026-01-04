#[cfg(target_os = "windows")]
pub mod kd_win32 {
    use crate::{InputError, KeyState, KeyboardEvent, MouseEvent, Result};
    use tracing::{debug, info};
    use windows::Win32::{
        UI::Input::KeyboardAndMouse::*,
        UI::WindowsAndMessaging::*,
    };
    use crate::{ButtonState, GamepadEvent, InputConfig, InputInjector, KeyCode, MouseButton};

    pub struct WindowsInputInjector {
        config: Option<InputConfig>,
    }

    impl WindowsInputInjector {
        pub fn new() -> Result<Self> {
            Ok(Self { config: None })
        }

        fn keycode_to_vk(&self, key: KeyCode) -> u16 {
            key as u16
        }
    }

    impl InputInjector for WindowsInputInjector {
        fn init(&mut self, config: InputConfig) -> Result<()> {
            info!("Initializing Windows input injection");
            self.config = Some(config);
            Ok(())
        }

        fn inject_keyboard(&mut self, event: KeyboardEvent) -> Result<()> {
            unsafe {
                let vk = self.keycode_to_vk(event.key);
                let scan_code = MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC);

                let mut flags = KEYBD_EVENT_FLAGS(0);
                if event.state == KeyState::Released {
                    flags = KEYEVENTF_KEYUP;
                }

                let mut input = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(vk),
                            wScan: scan_code as u16,
                            dwFlags: flags,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };

                let result = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);

                if result == 0 {
                    return Err(InputError::InjectionFailed("SendInput failed".into()));
                }

                debug!("Injected keyboard: {:?} {:?}", event.key, event.state);
                Ok(())
            }
        }

        fn inject_mouse(&mut self, event: MouseEvent) -> Result<()> {
            unsafe {
                let mut input = INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx: 0,
                            dy: 0,
                            mouseData: 0,
                            dwFlags: MOUSE_EVENT_FLAGS(0),
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };

                match event {
                    MouseEvent::Move { x, y, relative } => {
                        input.Anonymous.mi.dx = x;
                        input.Anonymous.mi.dy = y;
                        input.Anonymous.mi.dwFlags = if relative {
                            MOUSEEVENTF_MOVE
                        } else {
                            MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE
                        };
                    }
                    MouseEvent::Button { button, state } => {
                        input.Anonymous.mi.dwFlags = match (button, state) {
                            (MouseButton::Left, ButtonState::Pressed) => MOUSEEVENTF_LEFTDOWN,
                            (MouseButton::Left, ButtonState::Released) => MOUSEEVENTF_LEFTUP,
                            (MouseButton::Right, ButtonState::Pressed) => MOUSEEVENTF_RIGHTDOWN,
                            (MouseButton::Right, ButtonState::Released) => MOUSEEVENTF_RIGHTUP,
                            (MouseButton::Middle, ButtonState::Pressed) => MOUSEEVENTF_MIDDLEDOWN,
                            (MouseButton::Middle, ButtonState::Released) => MOUSEEVENTF_MIDDLEUP,
                            _ => MOUSE_EVENT_FLAGS(0),
                        };
                    }
                    MouseEvent::Wheel { delta_x: _, delta_y } => {
                        input.Anonymous.mi.mouseData = delta_y as u32;
                        input.Anonymous.mi.dwFlags = MOUSEEVENTF_WHEEL;
                    }
                }

                let result = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);

                if result == 0 {
                    return Err(InputError::InjectionFailed("SendInput failed".into()));
                }

                debug!("Injected mouse event");
                Ok(())
            }
        }

        fn inject_gamepad(&mut self, event: GamepadEvent) -> Result<()> {
            // TODO: Implement XInput/ViGEm virtual gamepad
            debug!("Gamepad injection not yet implemented on Windows");
            Ok(())
        }

        fn shutdown(&mut self) -> Result<()> {
            info!("Shutting down Windows input injection");
            Ok(())
        }
    }
}