#include <ntddk.h>
#include <wdf.h>
#include <vhf.h>
#include "public.h"

//
// HID Report Descriptor
//
// This byte array is the schema for our virtual device. Windows reads it
// once at startup and uses it forever after to interpret our reports.
//
// The format is a sequence of items. Each item is 1-5 bytes:
//   byte 0: tag (upper 4 bits) | type (bits 2-3) | size (lower 2 bits)
//   bytes 1-4: the value, if size > 0
//
// Types:
//   0 = Main    (defines inputs/outputs/features, opens/closes collections)
//   1 = Global  (applies to all following items until changed)
//   2 = Local   (applies only to the next main item)
//
// We define three report IDs in one descriptor:
//   Report ID 1 = Gamepad
//   Report ID 2 = Keyboard
//   Report ID 3 = Mouse
//

static UCHAR g_ReportDescriptor[] = {

    // ── Gamepad (Report ID 1) ─────────────────────────────────────────────────
    //
    // Usage Page and Usage identify what kind of device this collection is.
    // Windows uses these to decide which driver stack to load on top of ours.
    // Generic Desktop / Gamepad tells Windows this behaves like a gamepad.

    0x05, 0x01,     // Usage Page (Generic Desktop)
    0x09, 0x05,     // Usage (Gamepad)
    0xA1, 0x01,     // Collection (Application) — opens the gamepad collection

    0x85, 0x01,     //   Report ID (1) — all items until next Report ID belong here

    // Buttons: 14 buttons, 1 bit each
    // LOGICAL_MINIMUM/MAXIMUM define the value range: 0 (not pressed) or 1 (pressed)
    // REPORT_SIZE is bits per field, REPORT_COUNT is number of fields
    // So 14 fields of 1 bit = 14 bits total
    // INPUT (Data, Variable, Absolute) means:
    //   Data     = not constant (changes with user input)
    //   Variable = each bit is a separate button (not an array)
    //   Absolute = value is absolute state, not a delta

    0x05, 0x09,     //   Usage Page (Button)
    0x19, 0x01,     //   Usage Minimum (Button 1)
    0x29, 0x0E,     //   Usage Maximum (Button 14)
    0x15, 0x00,     //   Logical Minimum (0)
    0x25, 0x01,     //   Logical Maximum (1)
    0x75, 0x01,     //   Report Size (1 bit)
    0x95, 0x0E,     //   Report Count (14)
    0x81, 0x02,     //   Input (Data, Variable, Absolute)

    // Padding: 2 bits to reach a byte boundary (14 bits + 2 = 16 bits = 2 bytes)
    // INPUT (Constant) means this field is always 0 and carries no data

    0x75, 0x01,     //   Report Size (1 bit)
    0x95, 0x02,     //   Report Count (2)
    0x81, 0x03,     //   Input (Constant)

    // Left stick X axis
    // PHYSICAL_MINIMUM/MAXIMUM are optional hints about real-world units.
    // LOGICAL range -32768..32767 maps the full INT16 range.
    // Usage X is the standard HID usage for horizontal axis.

    0x05, 0x01,     //   Usage Page (Generic Desktop)
    0x09, 0x30,     //   Usage (X)  — left stick X
    0x15, 0x00,     //   Logical Minimum (0) — will be overridden below
    0x16, 0x00, 0x80, //  Logical Minimum (-32768) — 2-byte value
    0x26, 0xFF, 0x7F, //  Logical Maximum (32767)  — 2-byte value
    0x75, 0x10,     //   Report Size (16 bits)
    0x95, 0x01,     //   Report Count (1)
    0x81, 0x06,     //   Input (Data, Variable, Relative)

    // Left stick Y axis
    0x09, 0x31,     //   Usage (Y)  — left stick Y
    0x16, 0x00, 0x80, //  Logical Minimum (-32768)
    0x26, 0xFF, 0x7F, //  Logical Maximum (32767)
    0x75, 0x10,     //   Report Size (16 bits)
    0x95, 0x01,     //   Report Count (1)
    0x81, 0x06,     //   Input (Data, Variable, Relative)

    // Right stick X axis (Rx in HID terminology)
    0x09, 0x33,     //   Usage (Rx) — right stick X
    0x16, 0x00, 0x80, //  Logical Minimum (-32768)
    0x26, 0xFF, 0x7F, //  Logical Maximum (32767)
    0x75, 0x10,     //   Report Size (16 bits)
    0x95, 0x01,     //   Report Count (1)
    0x81, 0x06,     //   Input (Data, Variable, Relative)

    // Right stick Y axis (Ry in HID terminology)
    0x09, 0x34,     //   Usage (Ry) — right stick Y
    0x16, 0x00, 0x80, //  Logical Minimum (-32768)
    0x26, 0xFF, 0x7F, //  Logical Maximum (32767)
    0x75, 0x10,     //   Report Size (16 bits)
    0x95, 0x01,     //   Report Count (1)
    0x81, 0x06,     //   Input (Data, Variable, Relative)

    // Left trigger (Z axis, unsigned 0..255)
    // We use the Z axis usage for left trigger — common convention.
    0x09, 0x32,     //   Usage (Z)  — left trigger
    0x15, 0x00,     //   Logical Minimum (0)
    0x26, 0xFF, 0x00, //  Logical Maximum (255)
    0x75, 0x08,     //   Report Size (8 bits)
    0x95, 0x01,     //   Report Count (1)
    0x81, 0x02,     //   Input (Data, Variable, Absolute)

    // Right trigger (Rz axis, unsigned 0..255)
    0x09, 0x35,     //   Usage (Rz) — right trigger
    0x15, 0x00,     //   Logical Minimum (0)
    0x26, 0xFF, 0x00, //  Logical Maximum (255)
    0x75, 0x08,     //   Report Size (8 bits)
    0x95, 0x01,     //   Report Count (1)
    0x81, 0x02,     //   Input (Data, Variable, Absolute)

    0xC0,           // End Collection (closes the gamepad Application collection)

    // ── Keyboard (Report ID 2) ────────────────────────────────────────────────
    //
    // Standard USB HID boot protocol keyboard.
    // This is a well-known layout that every OS understands out of the box.
    // Usage Page 0x01 Generic Desktop, Usage 0x06 Keyboard.

    0x05, 0x01,     // Usage Page (Generic Desktop)
    0x09, 0x06,     // Usage (Keyboard)
    0xA1, 0x01,     // Collection (Application)

    0x85, 0x02,     //   Report ID (2)

    // Modifier keys: 8 bits, one per modifier key
    // Usage Page 7 = Keyboard/Keypad
    // Usage 0xE0..0xE7 = left/right ctrl/shift/alt/gui

    0x05, 0x07,     //   Usage Page (Keyboard/Keypad)
    0x19, 0xE0,     //   Usage Minimum (Left Control)
    0x29, 0xE7,     //   Usage Maximum (Right GUI)
    0x15, 0x00,     //   Logical Minimum (0)
    0x25, 0x01,     //   Logical Maximum (1)
    0x75, 0x01,     //   Report Size (1 bit)
    0x95, 0x08,     //   Report Count (8)
    0x81, 0x02,     //   Input (Data, Variable, Absolute)

    // Reserved byte — required by the boot protocol, always 0

    0x75, 0x08,     //   Report Size (8 bits)
    0x95, 0x01,     //   Report Count (1)
    0x81, 0x03,     //   Input (Constant)

    // 6 key slots: each is a HID usage code (0x00 = no key)
    // Array format: each slot contains the usage code of a pressed key.
    // Up to 6 simultaneous keys.

    0x05, 0x07,     //   Usage Page (Keyboard/Keypad)
    0x19, 0x00,     //   Usage Minimum (0)
    0x29, 0xFF,     //   Usage Maximum (255)
    0x15, 0x00,     //   Logical Minimum (0)
    0x26, 0xFF, 0x00, //  Logical Maximum (255)
    0x75, 0x08,     //   Report Size (8 bits)
    0x95, 0x06,     //   Report Count (6)
    0x81, 0x00,     //   Input (Data, Array, Absolute)

    0xC0,           // End Collection

    // ── Mouse (Report ID 3) ───────────────────────────────────────────────────
    //
    // Standard relative mouse. X and Y are deltas (not absolute positions).
    // This is what games using raw input receive — real relative movement,
    // not Win32 cursor position.

    0x05, 0x01,     // Usage Page (Generic Desktop)
    0x09, 0x02,     // Usage (Mouse)
    0xA1, 0x01,     // Collection (Application)

    0x85, 0x03,     //   Report ID (3)

    0x09, 0x01,     //   Usage (Pointer)
    0xA1, 0x00,     //   Collection (Physical) — groups pointer data

    // 3 mouse buttons: left, right, middle

    0x05, 0x09,     //     Usage Page (Button)
    0x19, 0x01,     //     Usage Minimum (Button 1 = left)
    0x29, 0x03,     //     Usage Maximum (Button 3 = middle)
    0x15, 0x00,     //     Logical Minimum (0)
    0x25, 0x01,     //     Logical Maximum (1)
    0x75, 0x01,     //     Report Size (1 bit)
    0x95, 0x03,     //     Report Count (3)
    0x81, 0x02,     //     Input (Data, Variable, Absolute)

    // 5 padding bits to reach a byte boundary

    0x75, 0x05,     //     Report Size (5 bits)
    0x95, 0x01,     //     Report Count (1)
    0x81, 0x03,     //     Input (Constant)

    // X movement: signed 16-bit relative delta
    // Relative means this is a delta from last position, not absolute.
    // Games using raw input receive this directly.

    0x05, 0x01,     //     Usage Page (Generic Desktop)
    0x09, 0x30,     //     Usage (X)
    0x16, 0x00, 0x80, //    Logical Minimum (-32768)
    0x26, 0xFF, 0x7F, //    Logical Maximum (32767)
    0x75, 0x10,     //     Report Size (16 bits)
    0x95, 0x01,     //     Report Count (1)
    0x81, 0x06,     //     Input (Data, Variable, Relative)

    // Y movement: signed 16-bit relative delta

    0x09, 0x31,     //     Usage (Y)
    0x16, 0x00, 0x80, //    Logical Minimum (-32768)
    0x26, 0xFF, 0x7F, //    Logical Maximum (32767)
    0x75, 0x10,     //     Report Size (16 bits)
    0x95, 0x01,     //     Report Count (1)
    0x81, 0x06,     //     Input (Data, Variable, Relative)

    // Scroll wheel: signed 8-bit relative delta

    0x09, 0x38,     //     Usage (Wheel)
    0x15, 0x81,     //     Logical Minimum (-127)
    0x25, 0x7F,     //     Logical Maximum (127)
    0x75, 0x08,     //     Report Size (8 bits)
    0x95, 0x01,     //     Report Count (1)
    0x81, 0x06,     //     Input (Data, Variable, Relative)

    0xC0,           //   End Collection (Physical)
    0xC0,           // End Collection (Application)
};

//
// VHF handle — our connection to the Virtual HID Framework.
// The VHF is what makes Windows treat our driver as a real HID device.
// We get this handle back from VhfCreate() and use it for all subsequent
// VHF calls.
//
VHFHANDLE g_VhfHandle = NULL;

//
// Current gamepad state
//
// The gamepad works differently from keyboard and mouse. A keyboard report
// says "these keys are currently pressed." A mouse report says "the mouse
// moved by this delta." A gamepad report says the FULL current state of
// every button and axis — even the ones that didn't change.
//
// So we maintain the current state here and update individual fields as
// events arrive, then submit the whole struct each time.
//
KD_GAMEPAD_REPORT g_GamepadState = { .report_id = 1 };

//
// KdSubmitGamepadReport
//
// Called from ioctl.c when kd-server sends IOCTL_KD_GAMEPAD_REPORT.
// Copies the incoming report into our state and submits it to VHF.
//
NTSTATUS KdSubmitGamepadReport(KD_GAMEPAD_REPORT* report)
{
    // Update global state
    g_GamepadState = *report;
    g_GamepadState.report_id = 1; // ensure it's always set correctly

    HID_XFER_PACKET packet;
    packet.reportBuffer = (PUCHAR)&g_GamepadState;
    packet.reportBufferLen = sizeof(g_GamepadState);
    packet.reportId = 1;

    return VhfReadReportSubmit(g_VhfHandle, &packet);
}

//
// KdSubmitKeyboardReport
//
// Called from ioctl.c when kd-server sends IOCTL_KD_KEYBOARD_REPORT.
// Keyboard reports are stateless — each report fully describes current
// pressed keys, so we just submit directly without maintaining state.
//
NTSTATUS KdSubmitKeyboardReport(KD_KEYBOARD_REPORT* report)
{
    report->report_id = 2;

    HID_XFER_PACKET packet;
    packet.reportBuffer = (PUCHAR)report;
    packet.reportBufferLen = sizeof(KD_KEYBOARD_REPORT);
    packet.reportId = 2;

    return VhfReadReportSubmit(g_VhfHandle, &packet);
}

//
// KdSubmitMouseReport
//
// Called from ioctl.c when kd-server sends IOCTL_KD_MOUSE_REPORT.
// Mouse reports are also stateless — each report is a delta, not a state.
//
NTSTATUS KdSubmitMouseReport(KD_MOUSE_REPORT* report)
{
    report->report_id = 3;

    HID_XFER_PACKET packet;
    packet.reportBuffer = (PUCHAR)report;
    packet.reportBufferLen = sizeof(KD_MOUSE_REPORT);
    packet.reportId = 3;

    return VhfReadReportSubmit(g_VhfHandle, &packet);
}

//
// KdCreateVhfDevice
//
// Called from driver.c during device initialization.
// Registers our virtual HID device with Windows via VHF.
// After this call succeeds, Windows sees a new HID device plugged in.
//
NTSTATUS KdCreateVhfDevice(WDFDEVICE wdfDevice)
{
    VHF_CONFIG config;
    VHF_CONFIG_INIT(
        &config,
        WdfDeviceWdmGetDeviceObject(wdfDevice),
        sizeof(g_ReportDescriptor),
        g_ReportDescriptor
    );

    // These identify our device to Windows.
    // You can see these in Device Manager.
    config.VendorID  = 0x4B44; // 'KD' in ASCII
    config.ProductID = 0x0001;
    config.VersionNumber = 0x0001;

    NTSTATUS status = VhfCreate(&config, &g_VhfHandle);
    if (!NT_SUCCESS(status))
    {
        KdPrint(("VhfCreate failed: 0x%08X\n", status));
        return status;
    }

    status = VhfStart(g_VhfHandle);
    if (!NT_SUCCESS(status))
    {
        KdPrint(("VhfStart failed: 0x%08X\n", status));
        VhfDelete(g_VhfHandle, TRUE);
        g_VhfHandle = NULL;
    }

    return status;
}

//
// KdDestroyVhfDevice
//
// Called from driver.c during device cleanup.
// Tells VHF we're done — Windows removes the virtual device.
//
VOID KdDestroyVhfDevice(VOID)
{
    if (g_VhfHandle != NULL) {
        VhfDelete(g_VhfHandle, TRUE);
        g_VhfHandle = NULL;
    }
}

//
// KdGetReportDescriptor / KdGetReportDescriptorSize
//
// Called from driver.c when Windows asks "what does your device look like?"
// Returns a pointer to and size of the descriptor defined above.
//
const UCHAR* KdGetReportDescriptor(VOID)
{
    return g_ReportDescriptor;
}

ULONG KdGetReportDescriptorSize(VOID)
{
    return sizeof(g_ReportDescriptor);
}