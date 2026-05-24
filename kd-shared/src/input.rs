use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum InputDevice
{
    MouseKeyboard,
    Gamepad,
    Touchscreen,
    Unknown
}