use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Copy, Clone)]
pub enum ScreenType {
    #[default]
    Home,
    Connect,
    Session
}

pub trait Screen {
    fn on_show(&mut self) {}
    fn render(&mut self, ui: &mut egui::Ui);
    fn get_type(&self) -> ScreenType;
}