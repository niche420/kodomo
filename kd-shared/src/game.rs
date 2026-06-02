use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameMetadata
{
    pub title: String
}
