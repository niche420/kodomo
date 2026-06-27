use std::any::Any;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Copy, Clone, PartialEq, Eq)]
pub enum ScreenType {
    #[default]
    Home,
    Handshake,
    Session,
}

pub trait Screen {
    fn on_show(&mut self) {}
    fn render(&mut self, ui: &mut egui::Ui);
    fn get_type(&self) -> ScreenType;
    fn as_any(&self) -> &dyn Any;
}