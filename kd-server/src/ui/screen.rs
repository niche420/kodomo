use std::any::Any;
use serde::{Deserialize, Serialize};
use crate::http::SharedState;
use crate::ui::connect::ConnectScreen;
use crate::ui::games::GamesScreen;
use crate::ui::session::SessionScreen;

#[derive(Serialize, Deserialize, Default, Copy, Clone, PartialEq, Eq)]
pub enum ScreenType {
    #[default]
    Games,
    Connect,
    Session
}

pub trait Screen {
    fn render(&mut self, ui: &mut egui::Ui);
    fn get_type(&self) -> ScreenType;
    fn on_show(&mut self) {}
}
