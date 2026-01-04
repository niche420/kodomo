use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(target_os = "windows")]
mod win32;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

pub mod events;
pub use events::*;

pub type Result<T> = std::result::Result<T, InputError>;

#[derive(Debug, Error)]
pub enum InputError {
    #[error("Platform not supported")]
    UnsupportedPlatform,

    #[error("Initialization failed: {0}")]
    InitFailed(String),

    #[error("Injection failed: {0}")]
    InjectionFailed(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// Configuration for input handling
#[derive(Debug, Clone)]
pub struct InputConfig {
    pub enable_keyboard: bool,
    pub enable_mouse: bool,
    pub enable_gamepad: bool,
    pub mouse_acceleration: f32,
    pub relative_mouse: bool, // Capture mouse for games
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            enable_keyboard: true,
            enable_mouse: true,
            enable_gamepad: true,
            mouse_acceleration: 1.0,
            relative_mouse: true,
        }
    }
}

/// Trait for platform-specific input injection
pub trait InputInjector: Send + Sync {
    fn init(&mut self, config: InputConfig) -> Result<()>;
    fn inject_keyboard(&mut self, event: KeyboardEvent) -> Result<()>;
    fn inject_mouse(&mut self, event: MouseEvent) -> Result<()>;
    fn inject_gamepad(&mut self, event: GamepadEvent) -> Result<()>;
    fn shutdown(&mut self) -> Result<()>;
}

// Platform selection at compile time
#[cfg(target_os = "windows")]
type PlatformInjector = win32::kd_win32::WindowsInputInjector;

#[cfg(target_os = "linux")]
type PlatformInjector = linux::LinuxInputInjector;

#[cfg(target_os = "macos")]
type PlatformInjector = macos::MacOSInputInjector;

/// Main input handler - automatically uses correct platform
pub struct InputHandler {
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    injector: PlatformInjector,
    config: InputConfig,
}

impl InputHandler {
    pub fn new() -> Result<Self> {
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        return Err(InputError::UnsupportedPlatform);

        #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
        {
            Ok(Self {
                injector: PlatformInjector::new()?,
                config: InputConfig::default(),
            })
        }
    }

    pub fn init(&mut self, config: InputConfig) -> Result<()> {
        self.config = config.clone();
        self.injector.init(config)
    }

    pub fn handle_event(&mut self, event: InputEvent) -> Result<()> {
        match event {
            InputEvent::Keyboard(e) if self.config.enable_keyboard => {
                self.injector.inject_keyboard(e)
            }
            InputEvent::Mouse(e) if self.config.enable_mouse => {
                self.injector.inject_mouse(e)
            }
            InputEvent::Gamepad(e) if self.config.enable_gamepad => {
                self.injector.inject_gamepad(e)
            }
            _ => Ok(()), // Event type disabled
        }
    }

    pub fn shutdown(&mut self) -> Result<()> {
        self.injector.shutdown()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyboard_event_serialization() {
        let event = KeyboardEvent {
            key: KeyCode::A,
            state: KeyState::Pressed,
            modifiers: KeyModifiers::default(),
            timestamp: 12345,
        };

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: KeyboardEvent = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.key, KeyCode::A);
        assert_eq!(deserialized.state, KeyState::Pressed);
    }

    #[test]
    fn test_input_event_enum() {
        let kb_event = InputEvent::Keyboard(KeyboardEvent {
            key: KeyCode::W,
            state: KeyState::Pressed,
            modifiers: KeyModifiers::default(),
            timestamp: 0,
        });

        match kb_event {
            InputEvent::Keyboard(e) => assert_eq!(e.key, KeyCode::W),
            _ => panic!("Wrong event type"),
        }
    }
}