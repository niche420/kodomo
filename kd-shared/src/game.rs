use serde::{Deserialize, Serialize};
use crate::input::InputDevice;

#[derive(Serialize, Deserialize)]
pub struct GameMetadata
{
    title: String,
    genre: Genre,
    input: InputDevice
}

#[derive(Serialize, Deserialize)]
pub enum Genre
{
    Shooter,
    Rpg,
    Platformer
}