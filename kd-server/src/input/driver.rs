use windows::core::GUID;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces,
    SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW,
    DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA,
    SP_DEVICE_INTERFACE_DETAIL_DATA_W,
};
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE,
    OPEN_EXISTING,
};
use windows::Win32::System::IO::DeviceIoControl;
use kd_shared::profile::{GamepadAxis, GamepadButton, GamepadTrigger};
use crate::input::InputInjector;

// IOCTL codes — must match public.h exactly
const IOCTL_KD_GAMEPAD_REPORT:  u32 = ctl_code(0x22, 0x800);
const IOCTL_KD_KEYBOARD_REPORT: u32 = ctl_code(0x22, 0x801);
const IOCTL_KD_MOUSE_REPORT:    u32 = ctl_code(0x22, 0x802);

// CTL_CODE(DeviceType, Function, Method=METHOD_BUFFERED, Access=FILE_ANY_ACCESS)
// METHOD_BUFFERED = 0, FILE_ANY_ACCESS = 0
const fn ctl_code(device_type: u32, function: u32) -> u32 {
    (device_type << 16) | (0 << 14) | (function << 2) | 0
}

// Must match KD_GAMEPAD_REPORT in public.h with #pragma pack(1)
#[repr(C, packed)]
struct KdGamepadReport {
    report_id:     u8,   // always 1
    buttons:       u16,
    left_x:        i16,
    left_y:        i16,
    right_x:       i16,
    right_y:       i16,
    left_trigger:  u8,
    right_trigger: u8,
}

// Must match KD_KEYBOARD_REPORT in public.h with #pragma pack(1)
#[repr(C, packed)]
struct KdKeyboardReport {
    report_id: u8,       // always 2
    modifiers: u8,
    reserved:  u8,       // always 0
    keycodes:  [u8; 6],
}

// Must match KD_MOUSE_REPORT in public.h with #pragma pack(1)
#[repr(C, packed)]
struct KdMouseReport {
    report_id: u8,       // always 3
    buttons:   u8,
    x:         i16,
    y:         i16,
    wheel:     i8,
}

// Gamepad button bit positions — must match the descriptor in report.c
const BTN_SOUTH:      u16 = 1 << 0;
const BTN_EAST:       u16 = 1 << 1;
const BTN_WEST:       u16 = 1 << 2;
const BTN_NORTH:      u16 = 1 << 3;
const BTN_LBUMPER:    u16 = 1 << 4;
const BTN_RBUMPER:    u16 = 1 << 5;
const BTN_LSTICK:     u16 = 1 << 6;
const BTN_RSTICK:     u16 = 1 << 7;
const BTN_DPAD_UP:    u16 = 1 << 8;
const BTN_DPAD_DOWN:  u16 = 1 << 9;
const BTN_DPAD_LEFT:  u16 = 1 << 10;
const BTN_DPAD_RIGHT: u16 = 1 << 11;
const BTN_START:      u16 = 1 << 12;
const BTN_SELECT:     u16 = 1 << 13;

struct SendHandle(HANDLE);
unsafe impl Send for SendHandle {}

pub struct DriverInjector {
    handle: SendHandle,
    /// Current full gamepad state. Updated on every axis/button event
    /// and submitted as a complete report each time.
    gamepad: KdGamepadReport,
    /// Current keyboard state.
    /// keycodes[0..6] hold currently pressed HID usage codes.
    keyboard: KdKeyboardReport,
    /// Current mouse button state. Mouse x/y/wheel are stateless deltas
    /// so we only persist the button bitmask.
    mouse_buttons: u8,
}

impl DriverInjector {
    /// Open the kd-input.sys device interface.
    /// Returns an error if the driver is not installed or not running.
    pub fn open() -> anyhow::Result<Self> {
        let handle = open_device_interface()?;
        Ok(Self {
            handle: SendHandle(handle),
            gamepad: KdGamepadReport {
                report_id: 1,
                buttons: 0,
                left_x: 0, left_y: 0,
                right_x: 0, right_y: 0,
                left_trigger: 0, right_trigger: 0,
            },
            keyboard: KdKeyboardReport {
                report_id: 2,
                modifiers: 0,
                reserved: 0,
                keycodes: [0u8; 6],
            },
            mouse_buttons: 0,
        })
    }

    fn send_gamepad(&mut self) -> anyhow::Result<()> {
        send_ioctl(self.handle.0, IOCTL_KD_GAMEPAD_REPORT, &self.gamepad)
    }

    fn send_keyboard(&mut self) -> anyhow::Result<()> {
        send_ioctl(self.handle.0, IOCTL_KD_KEYBOARD_REPORT, &self.keyboard)
    }

    fn send_mouse(&self, x: i16, y: i16, wheel: i8) -> anyhow::Result<()> {
        let report = KdMouseReport {
            report_id: 3,
            buttons: self.mouse_buttons,
            x, y, wheel,
        };
        send_ioctl(self.handle.0, IOCTL_KD_MOUSE_REPORT, &report)
    }

    /// Add a key usage code to the keyboard report's active slots.
    /// The keyboard report holds up to 6 simultaneously pressed keys.
    fn keyboard_press(&mut self, usage: u8) {
        // Find an empty slot and fill it
        for slot in self.keyboard.keycodes.iter_mut() {
            if *slot == 0 {
                *slot = usage;
                return;
            }
        }
        // All 6 slots full — ignore (6-key rollover limit)
    }

    /// Remove a key usage code from the keyboard report's active slots.
    fn keyboard_release(&mut self, usage: u8) {
        for slot in self.keyboard.keycodes.iter_mut() {
            if *slot == usage {
                *slot = 0;
                return;
            }
        }
    }

    fn gamepad_button_bit(button: &GamepadButton) -> u16 {
        match button {
            GamepadButton::South      => BTN_SOUTH,
            GamepadButton::East       => BTN_EAST,
            GamepadButton::West       => BTN_WEST,
            GamepadButton::North      => BTN_NORTH,
            GamepadButton::LBumper    => BTN_LBUMPER,
            GamepadButton::RBumper    => BTN_RBUMPER,
            GamepadButton::LStick     => BTN_LSTICK,
            GamepadButton::RStick     => BTN_RSTICK,
            GamepadButton::DPadUp     => BTN_DPAD_UP,
            GamepadButton::DPadDown   => BTN_DPAD_DOWN,
            GamepadButton::DPadLeft   => BTN_DPAD_LEFT,
            GamepadButton::DPadRight  => BTN_DPAD_RIGHT,
            GamepadButton::Start      => BTN_START,
            GamepadButton::Select     => BTN_SELECT,
        }
    }
}

impl Drop for DriverInjector {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.handle.0).ok(); }
    }
}

impl InputInjector for DriverInjector {
    fn key_down(&mut self, scan_code: u16) {
        // Convert scan code to HID usage code.
        // The scan code → HID usage mapping follows the HID Usage Tables
        // spec, Set 1 (PS/2 scan codes map to HID page 0x07 usages).
        if let Some(usage) = scan_to_hid_usage(scan_code) {
            if is_modifier(usage) {
                self.keyboard.modifiers |= modifier_bit(usage);
            } else {
                self.keyboard_press(usage);
            }
            self.send_keyboard().ok();
        }
    }

    fn key_up(&mut self, scan_code: u16) {
        if let Some(usage) = scan_to_hid_usage(scan_code) {
            if is_modifier(usage) {
                self.keyboard.modifiers &= !modifier_bit(usage);
            } else {
                self.keyboard_release(usage);
            }
            self.send_keyboard().ok();
        }
    }

    fn mouse_button_down(&mut self, button: u8) {
        self.mouse_buttons |= 1 << button;
        self.send_mouse(0, 0, 0).ok();
    }

    fn mouse_button_up(&mut self, button: u8) {
        self.mouse_buttons &= !(1 << button);
        self.send_mouse(0, 0, 0).ok();
    }

    fn mouse_move(&mut self, dx: f32, dy: f32) {
        // Scale the normalized delta to a reasonable pixel movement range.
        // The scaling factor controls sensitivity — tune as needed.
        let x = (dx * 10.0).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        let y = (dy * 10.0).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        self.send_mouse(x, y, 0).ok();
    }

    fn gamepad_axis(&mut self, axis: GamepadAxis, value: f32) {
        // Normalize -1.0..=1.0 to -32768..=32767
        let raw = (value * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        match axis {
            GamepadAxis::LeftX  => self.gamepad.left_x  = raw,
            GamepadAxis::LeftY  => self.gamepad.left_y  = raw,
            GamepadAxis::RightX => self.gamepad.right_x = raw,
            GamepadAxis::RightY => self.gamepad.right_y = raw,
        }
        self.send_gamepad().ok();
    }

    fn gamepad_button_down(&mut self, button: GamepadButton) {
        self.gamepad.buttons |= Self::gamepad_button_bit(&button);
        self.send_gamepad().ok();
    }

    fn gamepad_button_up(&mut self, button: GamepadButton) {
        self.gamepad.buttons &= !Self::gamepad_button_bit(&button);
        self.send_gamepad().ok();
    }

    fn gamepad_trigger(&mut self, trigger: GamepadTrigger, value: f32) {
        // Normalize 0.0..=1.0 to 0..=255
        let raw = (value * 255.0).clamp(0.0, 255.0) as u8;
        match trigger {
            GamepadTrigger::Left  => self.gamepad.left_trigger  = raw,
            GamepadTrigger::Right => self.gamepad.right_trigger = raw,
        }
        self.send_gamepad().ok();
    }
}

// ─── Device interface discovery ───────────────────────────────────────────────

// GUID must match GUID_DEVINTERFACE_KD_INPUT in public.h
const GUID_DEVINTERFACE_KD_INPUT: GUID = GUID::from_values(
    0x1280fb3f, 0x6d94, 0x46d9,
    [0x8e, 0x9d, 0x80, 0x82, 0x3b, 0x02, 0xc4, 0xdb],
);

fn open_device_interface() -> anyhow::Result<HANDLE> {
    unsafe {
        // Get a device info set for all present devices exposing our interface
        let dev_info = SetupDiGetClassDevsW(
            Some(&GUID_DEVINTERFACE_KD_INPUT),
            None,
            None,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )?;

        if dev_info.is_invalid() {
            return Err(anyhow::anyhow!("SetupDiGetClassDevsW returned invalid handle"));
        }

        // Enumerate interfaces — we expect exactly one
        let mut iface_data = SP_DEVICE_INTERFACE_DATA {
            cbSize: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            ..Default::default()
        };

        let found = SetupDiEnumDeviceInterfaces(
            dev_info,
            None,
            &GUID_DEVINTERFACE_KD_INPUT,
            0, // index 0 = first (and only) device
            &mut iface_data,
        );

        if let Some(err) = found.err() {
            SetupDiDestroyDeviceInfoList(dev_info).ok();
            return Err(anyhow::anyhow!(
                format!("kd-input.sys not found: {}", err.to_string())
            ));
        }

        // First call: get required buffer size
        let mut required_size = 0u32;
        let _ = SetupDiGetDeviceInterfaceDetailW(
            dev_info,
            &iface_data,
            None,
            0,
            Some(&mut required_size),
            None,
        );

        // Allocate buffer and get the device path
        let mut buf = vec![0u8; required_size as usize];
        let detail = buf.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
        (*detail).cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;

        SetupDiGetDeviceInterfaceDetailW(
            dev_info,
            &iface_data,
            Some(detail),
            required_size,
            None,
            None,
        )?;

        SetupDiDestroyDeviceInfoList(dev_info).ok();

        // Open a file handle to the device using the path we just got
        let path_ptr = (*detail).DevicePath.as_ptr();
        let handle = CreateFileW(
            windows::core::PCWSTR(path_ptr),
            // Read+Write access so we can send IOCTLs in both directions
            (windows::Win32::Storage::FileSystem::FILE_GENERIC_READ |
                windows::Win32::Storage::FileSystem::FILE_GENERIC_WRITE).0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        )?;

        if handle == INVALID_HANDLE_VALUE {
            return Err(anyhow::anyhow!("CreateFileW returned INVALID_HANDLE_VALUE"));
        }

        Ok(handle)
    }
}

// ─── IOCTL helper ─────────────────────────────────────────────────────────────

fn send_ioctl<T>(handle: HANDLE, code: u32, data: &T) -> anyhow::Result<()> {
    let input = data as *const T as *const std::ffi::c_void;
    let input_size = std::mem::size_of::<T>() as u32;
    let mut bytes_returned = 0u32;

    unsafe {
        DeviceIoControl(
            handle,
            code,
            Some(input),
            input_size,
            None,           // no output buffer
            0,
            Some(&mut bytes_returned),
            None,           // not overlapped
        )?;
    }

    Ok(())
}

// ─── Scan code → HID usage conversion ────────────────────────────────────────
//
// Scan codes (Set 1 / PS/2) → HID keyboard usage page (0x07) usage codes.
// Only the keys present in our sc:: module need entries here.
// Full table: https://www.usb.org/sites/default/files/hut1_3_0.pdf page 83

fn scan_to_hid_usage(scan: u16) -> Option<u8> {
    match scan {
        0x01 => Some(0x29), // Escape
        0x02 => Some(0x1E), // 1
        0x03 => Some(0x1F), // 2
        0x04 => Some(0x20), // 3
        0x05 => Some(0x21), // 4
        0x06 => Some(0x22), // 5
        0x07 => Some(0x23), // 6
        0x08 => Some(0x24), // 7
        0x09 => Some(0x25), // 8
        0x0A => Some(0x26), // 9
        0x0B => Some(0x27), // 0
        0x1C => Some(0x28), // Return
        0x39 => Some(0x2C), // Space
        0x1D => Some(0xE0), // Left Ctrl  (modifier)
        0x2A => Some(0xE1), // Left Shift (modifier)
        0x38 => Some(0xE2), // Left Alt   (modifier)
        0x9D => Some(0xE4), // Right Ctrl  (extended, modifier)
        0x36 => Some(0xE5), // Right Shift (modifier)
        0xB8 => Some(0xE6), // Right Alt   (extended, modifier)
        // A-Z
        0x1E => Some(0x04), // A
        0x30 => Some(0x05), // B
        0x2E => Some(0x06), // C
        0x20 => Some(0x07), // D
        0x12 => Some(0x08), // E
        0x21 => Some(0x09), // F
        0x22 => Some(0x0A), // G
        0x23 => Some(0x0B), // H
        0x17 => Some(0x0C), // I
        0x24 => Some(0x0D), // J
        0x25 => Some(0x0E), // K
        0x26 => Some(0x0F), // L
        0x32 => Some(0x10), // M
        0x31 => Some(0x11), // N
        0x18 => Some(0x12), // O
        0x19 => Some(0x13), // P
        0x10 => Some(0x14), // Q
        0x13 => Some(0x15), // R
        0x1F => Some(0x16), // S
        0x14 => Some(0x17), // T
        0x16 => Some(0x18), // U
        0x2F => Some(0x19), // V
        0x11 => Some(0x1A), // W
        0x2D => Some(0x1B), // X
        0x15 => Some(0x1C), // Y
        0x2C => Some(0x1D), // Z
        // Arrow keys
        0x4B => Some(0x50), // Left
        0x4D => Some(0x4F), // Right
        0x48 => Some(0x52), // Up
        0x50 => Some(0x51), // Down
        // Function keys
        0x3B => Some(0x3A), // F1
        0x3C => Some(0x3B), // F2
        0x3D => Some(0x3C), // F3
        0x3E => Some(0x3D), // F4
        0x3F => Some(0x3E), // F5
        0x40 => Some(0x3F), // F6
        0x41 => Some(0x40), // F7
        0x42 => Some(0x41), // F8
        0x43 => Some(0x42), // F9
        0x44 => Some(0x43), // F10
        _ => None,
    }
}

// HID usage codes 0xE0-0xE7 are modifier keys
fn is_modifier(usage: u8) -> bool {
    usage >= 0xE0 && usage <= 0xE7
}

// Maps modifier HID usage to the bit position in the modifiers byte
fn modifier_bit(usage: u8) -> u8 {
    1 << (usage - 0xE0)
}