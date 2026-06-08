#pragma once

#include <initguid.h>

//
// Device interface GUID
// kd-server uses this to find the driver via SetupDiGetClassDevs.
// Must be unique — generated once, never changed.
//
// {4D36E96B-E325-11CE-BFC1-08002BE10318}
DEFINE_GUID(GUID_DEVINTERFACE_KD_INPUT,
    0x4d36e96b, 0xe325, 0x11ce,
    0xbf, 0xc1, 0x08, 0x00, 0x2b, 0xe1, 0x03, 0x18);

//
// IOCTL codes
//
// CTL_CODE(DeviceType, Function, Method, Access)
//   DeviceType: FILE_DEVICE_UNKNOWN (0x22) for software-only devices
//   Function:   0x800+ is the user-defined range
//   Method:     METHOD_BUFFERED — OS copies data between user/kernel buffers for us
//   Access:     FILE_ANY_ACCESS — no special privileges required
//
#define IOCTL_KD_GAMEPAD_REPORT  CTL_CODE(FILE_DEVICE_UNKNOWN, 0x800, METHOD_BUFFERED, FILE_ANY_ACCESS)
#define IOCTL_KD_KEYBOARD_REPORT CTL_CODE(FILE_DEVICE_UNKNOWN, 0x801, METHOD_BUFFERED, FILE_ANY_ACCESS)
#define IOCTL_KD_MOUSE_REPORT    CTL_CODE(FILE_DEVICE_UNKNOWN, 0x802, METHOD_BUFFERED, FILE_ANY_ACCESS)

//
// Report structs
//
// These must match the HID report descriptor in report.c exactly —
// every field maps to a specific byte range that Windows already knows
// about from reading the descriptor at device startup.
//
// #pragma pack(1) ensures no padding bytes are inserted between fields.
// Without it, the compiler might add padding for alignment and the byte
// offsets would no longer match what Windows expects.
//

#pragma pack(push, 1)

//
// Gamepad report (Report ID 1)
//
// buttons: bitmask of 14 buttons in the low 14 bits, 2 bits padding
//   bit 0  = South  (A)
//   bit 1  = East   (B)
//   bit 2  = West   (X)
//   bit 3  = North  (Y)
//   bit 4  = LBumper
//   bit 5  = RBumper
//   bit 6  = LStick click
//   bit 7  = RStick click
//   bit 8  = DPadUp
//   bit 9  = DPadDown
//   bit 10 = DPadLeft
//   bit 11 = DPadRight
//   bit 12 = Start
//   bit 13 = Select
//   bit 14-15 = padding
//
// axes: signed 16-bit, -32768..32767
// triggers: unsigned 8-bit, 0..255
//
typedef struct {
    UINT8  report_id;       // always 1
    UINT16 buttons;
    INT16  left_x;
    INT16  left_y;
    INT16  right_x;
    INT16  right_y;
    UINT8  left_trigger;
    UINT8  right_trigger;
} KD_GAMEPAD_REPORT;

//
// Keyboard report (Report ID 2)
//
// Standard USB HID boot protocol keyboard report.
// modifiers: bitmask
//   bit 0 = Left Ctrl
//   bit 1 = Left Shift
//   bit 2 = Left Alt
//   bit 3 = Left GUI
//   bit 4 = Right Ctrl
//   bit 5 = Right Shift
//   bit 6 = Right Alt
//   bit 7 = Right GUI
//
// keycodes: up to 6 simultaneous keys by USB HID usage code
//   (not scan codes — HID usage codes are different)
//   0x00 = no key / rollover
//
typedef struct {
    UINT8 report_id;        // always 2
    UINT8 modifiers;
    UINT8 reserved;         // always 0
    UINT8 keycodes[6];
} KD_KEYBOARD_REPORT;

//
// Mouse report (Report ID 3)
//
// buttons: bitmask
//   bit 0 = left button
//   bit 1 = right button
//   bit 2 = middle button
//
// x, y: signed 16-bit relative movement deltas
// wheel: signed 8-bit scroll wheel delta
//
typedef struct {
    UINT8 report_id;        // always 3
    UINT8 buttons;
    INT16 x;
    INT16 y;
    INT8  wheel;
} KD_MOUSE_REPORT;

#pragma pack(pop)