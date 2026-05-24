use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use crate::input::InputDevice;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameMetadata
{
    pub title: String
}
