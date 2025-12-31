#[cfg(target_os = "macos")]
pub mod macos {
    use super::*;
    use core_graphics::event::*;
    use tracing::{debug, info};

    pub struct MacOSInputInjector {
        config: Option<InputConfig>,
    }

    impl MacOSInputInjector {
        pub fn new() -> Result<Self> {
            Ok(Self { config: None })
        }
    }

    impl InputInjector for MacOSInputInjector {
        fn init(&mut self, config: InputConfig) -> Result<()> {
            info!("Initializing macOS input injection");
            self.config = Some(config);
            Ok(())
        }

        fn inject_keyboard(&mut self, event: KeyboardEvent) -> Result<()> {
            let keycode = CGKeyCode::from(event.key as u16);
            let key_down = event.state == KeyState::Pressed;

            if let Some(cg_event) = CGEvent::new_keyboard_event(
                CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
                    .map_err(|e| InputError::InjectionFailed(format!("Event source: {:?}", e)))?,
                keycode,
                key_down,
            ) {
                cg_event.post(CGEventTapLocation::HID);
                debug!("Injected keyboard: {:?} {:?}", event.key, event.state);
                Ok(())
            } else {
                Err(InputError::InjectionFailed("Failed to create CGEvent".into()))
            }
        }

        fn inject_mouse(&mut self, event: MouseEvent) -> Result<()> {
            let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
                .map_err(|e| InputError::InjectionFailed(format!("Event source: {:?}", e)))?;

            match event {
                MouseEvent::Move { x, y, relative: _ } => {
                    if let Some(cg_event) = CGEvent::new_mouse_event(
                        source,
                        CGEventType::MouseMoved,
                        CGPoint::new(x as f64, y as f64),
                        CGMouseButton::Left,
                    ) {
                        cg_event.post(CGEventTapLocation::HID);
                    }
                }
                MouseEvent::Button { button, state } => {
                    let (event_type, cg_button) = match (button, state) {
                        (MouseButton::Left, ButtonState::Pressed) =>
                            (CGEventType::LeftMouseDown, CGMouseButton::Left),
                        (MouseButton::Left, ButtonState::Released) =>
                            (CGEventType::LeftMouseUp, CGMouseButton::Left),
                        (MouseButton::Right, ButtonState::Pressed) =>
                            (CGEventType::RightMouseDown, CGMouseButton::Right),
                        (MouseButton::Right, ButtonState::Released) =>
                            (CGEventType::RightMouseUp, CGMouseButton::Right),
                        _ => return Ok(()),
                    };

                    if let Some(cg_event) = CGEvent::new_mouse_event(
                        source,
                        event_type,
                        CGPoint::new(0.0, 0.0),
                        cg_button,
                    ) {
                        cg_event.post(CGEventTapLocation::HID);
                    }
                }
                MouseEvent::Wheel { delta_x: _, delta_y } => {
                    if let Some(cg_event) = CGEvent::new_scroll_event(
                        source,
                        ScrollEventUnit::PIXEL,
                        1,
                        delta_y,
                        0,
                        0,
                    ) {
                        cg_event.post(CGEventTapLocation::HID);
                    }
                }
            }

            debug!("Injected mouse event");
            Ok(())
        }

        fn inject_gamepad(&mut self, event: GamepadEvent) -> Result<()> {
            debug!("Gamepad injection not yet implemented on macOS");
            Ok(())
        }

        fn shutdown(&mut self) -> Result<()> {
            info!("Shutting down macOS input injection");
            Ok(())
        }
    }
}