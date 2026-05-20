use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum InputDevice
{
    MouseKeyboard,
    Gamepad,
    Touchscreen,
    Unknown
}