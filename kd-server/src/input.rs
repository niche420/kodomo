mod stub;

use kd_shared::profile::{GameProfile, InputEvent, InputEventKind, PhysicalInput};

/// Injects a physical input on the host.
pub trait InputInjector: Send {
    fn key_down(&mut self, scan_code: u16);
    fn key_up(&mut self, scan_code: u16);
    fn mouse_button_down(&mut self, button: u8);
    fn mouse_button_up(&mut self, button: u8);
    fn mouse_move(&mut self, dx: f32, dy: f32);
    fn gamepad_axis(&mut self, axis: kd_shared::profile::GamepadAxis, value: f32);
    fn gamepad_button_down(&mut self, button: kd_shared::profile::GamepadButton);
    fn gamepad_button_up(&mut self, button: kd_shared::profile::GamepadButton);
    fn gamepad_trigger(&mut self, trigger: kd_shared::profile::GamepadTrigger, value: f32);
}

pub fn create_injector() -> Box<dyn InputInjector> {
    // Swap stub::StubInjector for the real driver injector once kd-input.sys exists
    Box::new(stub::StubInjector)
}

/// Routes an incoming InputEvent to the correct injector method.
pub fn dispatch(injector: &mut dyn InputInjector, event: &InputEvent, profile: &GameProfile) {
    let Some(action) = profile.action_for_slot(&event.action_id)
        .or_else(|| profile.actions.iter().find(|a| a.id == event.action_id))
    else {
        eprintln!("input: no action for id '{}'", event.action_id);
        return;
    };

    match (&action.input, &event.kind) {
        (PhysicalInput::Key(sc), InputEventKind::ButtonPress)    => injector.key_down(*sc),
        (PhysicalInput::Key(sc), InputEventKind::ButtonRelease)  => injector.key_up(*sc),

        (PhysicalInput::MouseButton(btn), InputEventKind::ButtonPress)   => injector.mouse_button_down(*btn),
        (PhysicalInput::MouseButton(btn), InputEventKind::ButtonRelease) => injector.mouse_button_up(*btn),

        (PhysicalInput::MouseAxis(axis), InputEventKind::Analog(v)) => {
            match axis {
                kd_shared::profile::MouseAxis::X => injector.mouse_move(*v, 0.0),
                kd_shared::profile::MouseAxis::Y => injector.mouse_move(0.0, *v),
            }
        }

        (PhysicalInput::GamepadAxis(axis), InputEventKind::Analog(v)) =>
            injector.gamepad_axis(axis.clone(), *v),

        (PhysicalInput::GamepadButton(btn), InputEventKind::ButtonPress) =>
            injector.gamepad_button_down(btn.clone()),
        (PhysicalInput::GamepadButton(btn), InputEventKind::ButtonRelease) =>
            injector.gamepad_button_up(btn.clone()),

        (PhysicalInput::GamepadTrigger(trig), InputEventKind::Analog(v)) =>
            injector.gamepad_trigger(trig.clone(), *v),

        _ => eprintln!("input: mismatched event kind for action '{}'", action.id),
    }
}