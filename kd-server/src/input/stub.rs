use kd_shared::profile::{GamepadAxis, GamepadButton, GamepadTrigger};
use crate::input::InputInjector;

pub struct StubInjector;

impl InputInjector for StubInjector {
    fn key_down(&mut self, scan_code: u16) {
        eprintln!("stub: key_down sc=0x{:02X}", scan_code);
    }
    fn key_up(&mut self, scan_code: u16) {
        eprintln!("stub: key_up sc=0x{:02X}", scan_code);
    }
    fn mouse_button_down(&mut self, button: u8) {
        eprintln!("stub: mouse_button_down btn={}", button);
    }
    fn mouse_button_up(&mut self, button: u8) {
        eprintln!("stub: mouse_button_up btn={}", button);
    }
    fn mouse_move(&mut self, dx: f32, dy: f32) {
        eprintln!("stub: mouse_move dx={:.2} dy={:.2}", dx, dy);
    }
    fn gamepad_axis(&mut self, axis: GamepadAxis, value: f32) {
        eprintln!("stub: gamepad_axis {:?} = {:.2}", axis, value);
    }
    fn gamepad_button_down(&mut self, button: GamepadButton) {
        eprintln!("stub: gamepad_button_down {:?}", button);
    }
    fn gamepad_button_up(&mut self, button: GamepadButton) {
        eprintln!("stub: gamepad_button_up {:?}", button);
    }
    fn gamepad_trigger(&mut self, trigger: GamepadTrigger, value: f32) {
        eprintln!("stub: gamepad_trigger {:?} = {:.2}", trigger, value);
    }
}