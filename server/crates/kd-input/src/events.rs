use super::*;

/// Top-level input event enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEvent {
    Keyboard(KeyboardEvent),
    Mouse(MouseEvent),
    Gamepad(GamepadEvent),
}

// Keyboard Events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardEvent {
    pub key: KeyCode,
    pub state: KeyState,
    pub modifiers: KeyModifiers,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool, // Windows key / Command key
}

impl Default for KeyModifiers {
    fn default() -> Self {
        Self {
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
        }
    }
}

/// Virtual key codes (similar to Windows VK codes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
pub enum KeyCode {
    // Letters
    A = 0x41, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,

    // Numbers
    Key0 = 0x30, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9,

    // Function keys
    F1 = 0x70, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,

    // Special keys
    Escape = 0x1B,
    Tab = 0x09,
    CapsLock = 0x14,
    Shift = 0x10,
    Control = 0x11,
    Alt = 0x12,
    Space = 0x20,
    Enter = 0x0D,
    Backspace = 0x08,

    // Arrow keys
    Left = 0x25,
    Up = 0x26,
    Right = 0x27,
    Down = 0x28,

    // Editing keys
    Insert = 0x2D,
    Delete = 0x2E,
    Home = 0x24,
    End = 0x23,
    PageUp = 0x21,
    PageDown = 0x22,
}

// Mouse Events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseEvent {
    Move { x: i32, y: i32, relative: bool },
    Button { button: MouseButton, state: ButtonState },
    Wheel { delta_x: i32, delta_y: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtonState {
    Pressed,
    Released,
}

// Gamepad Events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamepadEvent {
    pub gamepad_id: u32,
    pub event_type: GamepadEventType,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GamepadEventType {
    Button { button: GamepadButton, value: f32 },
    Axis { axis: GamepadAxis, value: f32 },
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GamepadButton {
    // Face buttons (Xbox layout)
    South,      // A / Cross
    East,       // B / Circle
    West,       // X / Square
    North,      // Y / Triangle

    // Shoulder buttons
    LeftShoulder,   // LB / L1
    RightShoulder,  // RB / R1
    LeftTrigger,    // LT / L2
    RightTrigger,   // RT / R2

    // Thumbsticks
    LeftThumb,
    RightThumb,

    // D-Pad
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,

    // Menu buttons
    Start,
    Select,
    Guide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GamepadAxis {
    LeftStickX,
    LeftStickY,
    RightStickX,
    RightStickY,
    LeftTrigger,
    RightTrigger,
}