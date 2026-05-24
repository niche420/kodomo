use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Copy, Clone)]
pub enum ScreenType {
    #[default]
    Home,
    Connect
}

pub trait Screen {
    fn render(&mut self, ui: &mut egui::Ui);
}