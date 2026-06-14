use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum PhysicalInput {
    Key(u16),
    MouseButton(u8),
    MouseAxis(MouseAxis),
    GamepadButton(GamepadButton),
    GamepadAxis(GamepadAxis),
    GamepadTrigger(GamepadTrigger),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MouseAxis { X, Y }

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum GamepadButton {
    South, East, West, North,
    LBumper, RBumper,
    LStick, RStick,
    DPadUp, DPadDown, DPadLeft, DPadRight,
    Start, Select,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum GamepadAxis { LeftX, LeftY, RightX, RightY }

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum GamepadTrigger { Left, Right }

/// Scan codes
pub mod sc {
    pub const LEFT:   u16 = 0x4B;
    pub const RIGHT:  u16 = 0x4D;
    pub const UP:     u16 = 0x48;
    pub const DOWN:   u16 = 0x50;
    pub const SPACE:  u16 = 0x39;
    pub const RETURN: u16 = 0x1C;
    pub const ESCAPE: u16 = 0x01;
    pub const LSHIFT: u16 = 0x2A;
    pub const LCTRL:  u16 = 0x1D;
    pub const LALT:   u16 = 0x38;
    pub const A: u16 = 0x1E;
    pub const B: u16 = 0x30;
    pub const C: u16 = 0x2E;
    pub const D: u16 = 0x20;
    pub const E: u16 = 0x12;
    pub const F: u16 = 0x21;
    pub const G: u16 = 0x22;
    pub const H: u16 = 0x23;
    pub const I: u16 = 0x17;
    pub const J: u16 = 0x24;
    pub const K: u16 = 0x25;
    pub const L: u16 = 0x26;
    pub const M: u16 = 0x32;
    pub const N: u16 = 0x31;
    pub const O: u16 = 0x18;
    pub const P: u16 = 0x19;
    pub const Q: u16 = 0x10;
    pub const R: u16 = 0x13;
    pub const S: u16 = 0x1F;
    pub const T: u16 = 0x14;
    pub const U: u16 = 0x16;
    pub const V: u16 = 0x2F;
    pub const W: u16 = 0x11;
    pub const X: u16 = 0x2D;
    pub const Y: u16 = 0x15;
    pub const Z: u16 = 0x2C;
    pub const F1:  u16 = 0x3B;
    pub const F2:  u16 = 0x3C;
    pub const F3:  u16 = 0x3D;
    pub const F4:  u16 = 0x3E;
    pub const F5:  u16 = 0x3F;
    pub const F6:  u16 = 0x40;
    pub const F7:  u16 = 0x41;
    pub const F8:  u16 = 0x42;
    pub const F9:  u16 = 0x43;
    pub const F10: u16 = 0x44;
}

// --- Touch widgets ---

/// Normalized layout rect. (0,0) = top-left, (1,1) = bottom-right.
/// Client multiplies by actual screen dimensions at render time.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// How a joystick widget produces output.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum JoystickMode {
    /// Continuous analog values -> gamepad stick axes.
    GamepadStick { x: GamepadAxis, y: GamepadAxis },
    /// Continuous deltas -> relative mouse movement (camera look).
    MouseLook,
    /// Snaps to 4 directions -> discrete press/release events, like a round DPad.
    Directional,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind")]
pub enum TouchWidget {
    Button  { id: String, label: String, rect: Rect },
    DPad    { id: String, rect: Rect },
    Joystick { id: String, rect: Rect, mode: JoystickMode },
    Trigger { id: String, label: String, rect: Rect },
}

impl TouchWidget {
    pub fn id(&self) -> &str {
        match self {
            Self::Button   { id, .. } => id,
            Self::DPad     { id, .. } => id,
            Self::Joystick { id, .. } => id,
            Self::Trigger  { id, .. } => id,
        }
    }
}

// --- Actions and bindings ---

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Action {
    pub id: String,
    pub label: String,
    pub input: PhysicalInput,
}

/// Maps a widget slot to an action.
///
/// Slot naming convention:
///   Button / Trigger          -> widget id
///   DPad                      -> "{id}_up/down/left/right"
///   Joystick (Directional)    -> "{id}_up/down/left/right"
///   Joystick (GamepadStick /
///             MouseLook)      -> "{id}_x" / "{id}_y"
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Binding {
    pub widget_slot: String,
    pub action_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GameProfile {
    pub game_title: String,
    pub widgets: Vec<TouchWidget>,
    pub actions: Vec<Action>,
    pub bindings: Vec<Binding>,
}

impl GameProfile {
    pub fn new(game_title: impl Into<String>) -> Self {
        Self {
            game_title: game_title.into(),
            widgets: vec![],
            actions: vec![],
            bindings: vec![],
        }
    }

    pub fn action_for_slot(&self, slot: &str) -> Option<&Action> {
        let binding = self.bindings.iter().find(|b| b.widget_slot == slot)?;
        self.actions.iter().find(|a| a.id == binding.action_id)
    }
}

// --- Wire format (UDP, client → server) ---

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct InputEvent {
    pub action_id: String,
    pub kind: InputEventKind,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum InputEventKind {
    ButtonPress,
    ButtonRelease,
    /// Joystick axes and trigger squeeze: -1.0..=1.0 (triggers: 0.0..=1.0)
    Analog(f32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut p = GameProfile::new("Test Game");
        p.widgets.push(TouchWidget::Button {
            id: "fire".into(),
            label: "FIRE".into(),
            rect: Rect { x: 0.8, y: 0.8, w: 0.1, h: 0.1 },
        });
        p.actions.push(Action {
            id: "fire".into(),
            label: "Fire".into(),
            input: PhysicalInput::MouseButton(0),
        });
        p.bindings.push(Binding {
            widget_slot: "fire".into(),
            action_id: "fire".into(),
        });
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(p, serde_json::from_str(&json).unwrap());
    }

    #[test]
    fn analog_event_roundtrip() {
        let ev = InputEvent {
            action_id: "look_x".into(),
            kind: InputEventKind::Analog(0.75),
        };
        let json = serde_json::to_string(&ev).unwrap();
        assert_eq!(ev, serde_json::from_str(&json).unwrap());
    }
}