use serde::{Deserialize, Serialize};
use crate::game::Genre;

#[derive(Serialize, Deserialize)]
pub struct ConnectParams
{
    ip: String,
    port: u16,
    session: String,
    game: String,
    genre: Genre
}