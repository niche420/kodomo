use serde::{Deserialize, Serialize};
use uuid::{uuid, Uuid};
use crate::game::GameMetadata;

#[derive(Serialize, Deserialize)]
pub struct SessionInfo
{
    token: Uuid,
    video_port: u16,
    input_port: u16,
    game: GameMetadata
}

impl SessionInfo
{
    pub fn new(video_port: u16, input_port: u16, game: GameMetadata) -> Self
    {
        Self {
            token: Uuid::new_v4(),
            video_port,
            input_port,
            game,
        }
    }
}