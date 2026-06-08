#include <ntddk.h>
#include <wdf.h>
#include <vhf.h>
#include "public.h"

//
// Forward declarations of functions defined in this file and ioctl.c
//
DRIVER_INITIALIZE DriverEntry;
EVT_WDF_DRIVER_DEVICE_ADD KdEvtDeviceAdd;
EVT_WDF_DEVICE_PREPARE_HARDWARE KdEvtDevicePrepareHardware;
EVT_WDF_DEVICE_RELEASE_HARDWARE KdEvtDeviceReleaseHardware;

// Declared in report.c
NTSTATUS KdCreateVhfDevice(WDFDEVICE wdfDevice);
VOID     KdDestroyVhfDevice(VOID);

// Declared in ioctl.c
VOID KdEvtIoDeviceControl(
    WDFQUEUE Queue,
    WDFREQUEST Request,
    size_t OutputBufferLength,
    size_t InputBufferLength,
    ULONG IoControlCode
);

//
// DriverEntry
//
// This is the first function Windows calls when the driver is loaded.
// It's the kernel equivalent of main(). 
//
// Our job here is minimal — just tell KMDF what function to call when
// a device is added. KMDF handles everything else about driver initialization.
//
// Parameters:
//   DriverObject  — Windows' internal representation of our driver.
//                   We pass it to WdfDriverCreate which wraps it in a
//                   WDF object we can use more safely.
//   RegistryPath  — path to our driver's registry key under
//                   HKLM\SYSTEM\CurrentControlSet\Services\kd-input
//                   We don't use it but KMDF needs it.
//
NTSTATUS DriverEntry(
    _In_ PDRIVER_OBJECT  DriverObject,
    _In_ PUNICODE_STRING RegistryPath
)
{
    WDF_DRIVER_CONFIG config;

    //
    // WDF_DRIVER_CONFIG_INIT fills the config struct with defaults and
    // registers our EvtDeviceAdd callback. When Windows detects that our
    // device needs to be created (because of the INF file telling it to
    // load us), it calls KdEvtDeviceAdd.
    //
    WDF_DRIVER_CONFIG_INIT(&config, KdEvtDeviceAdd);

    //
    // WdfDriverCreate turns the raw DRIVER_OBJECT into a managed WDF
    // driver object. The WDF_NO_OBJECT_ATTRIBUTES and WDF_NO_HANDLE
    // mean we don't need a custom attributes struct or a handle back.
    //
    NTSTATUS status = WdfDriverCreate(
        DriverObject,
        RegistryPath,
        WDF_NO_OBJECT_ATTRIBUTES,
        &config,
        WDF_NO_HANDLE
    );

    if (!NT_SUCCESS(status)) {
        // KdPrint is the kernel equivalent of printf — goes to a debug
        // output viewer like DebugView or WinDbg.
        KdPrint(("kd-input: WdfDriverCreate failed: 0x%x\n", status));
    }

    return status;
}

//
// KdEvtDeviceAdd
//
// Called by KMDF when Windows wants to add our device to the system.
// This happens once when the driver is first loaded.
//
// Our job here:
//   1. Create a WDF device object
//   2. Register a device interface (the GUID kd-server uses to find us)
//   3. Create an I/O queue to receive IOCTLs from kd-server
//   4. Create the VHF virtual HID device
//
// Parameters:
//   Driver      — our WDF driver object (created in DriverEntry)
//   DeviceInit  — initialization parameters for the new device.
//                 We configure it before calling WdfDeviceCreate.
//
NTSTATUS KdEvtDeviceAdd(
    _In_    WDFDRIVER       Driver,
    _Inout_ PWDFDEVICE_INIT DeviceInit
)
{
    UNREFERENCED_PARAMETER(Driver);
    NTSTATUS  status;
    WDFDEVICE device;

    //
    // Set up power management callbacks.
    //
    // PrepareHardware is called when the device is powered on.
    // ReleaseHardware is called when the device is powered off or removed.
    //
    // We use PrepareHardware to create the VHF device (making it visible
    // to Windows as a HID device) and ReleaseHardware to destroy it.
    //
    WDF_PNPPOWER_EVENT_CALLBACKS pnpPowerCallbacks;
    WDF_PNPPOWER_EVENT_CALLBACKS_INIT(&pnpPowerCallbacks);
    pnpPowerCallbacks.EvtDevicePrepareHardware = KdEvtDevicePrepareHardware;
    pnpPowerCallbacks.EvtDeviceReleaseHardware = KdEvtDeviceReleaseHardware;
    WdfDeviceInitSetPnpPowerEventCallbacks(DeviceInit, &pnpPowerCallbacks);

    //
    // Create the WDF device object.
    //
    // WDF_NO_OBJECT_ATTRIBUTES means we don't need to store any extra
    // data on the device object itself. WDF_NO_OBJECT_ATTRIBUTES is a
    // macro that passes NULL with the right type.
    //
    status = WdfDeviceCreate(&DeviceInit, WDF_NO_OBJECT_ATTRIBUTES, &device);
    if (!NT_SUCCESS(status)) {
        KdPrint(("kd-input: WdfDeviceCreate failed: 0x%x\n", status));
        return status;
    }

    //
    // Register our device interface.
    //
    // This is what makes our driver findable from userspace. kd-server
    // calls SetupDiGetClassDevs with GUID_DEVINTERFACE_KD_INPUT and gets
    // back a handle to our device. Without this, kd-server has no way to
    // open our driver.
    //
    status = WdfDeviceCreateDeviceInterface(
        device,
        &GUID_DEVINTERFACE_KD_INPUT,
        NULL  // no reference string needed
    );
    if (!NT_SUCCESS(status)) {
        KdPrint(("kd-input: WdfDeviceCreateDeviceInterface failed: 0x%x\n", status));
        return status;
    }

    //
    // Create the I/O queue.
    //
    // A queue is how WDF delivers I/O requests (IOCTLs from kd-server)
    // to our driver. We use a sequential queue — requests are delivered
    // one at a time, and the next one isn't delivered until we complete
    // the current one. This is the simplest and safest option for our use
    // case since we process IOCTLs very quickly.
    //
    // The alternative is a parallel queue where multiple requests can be
    // in-flight simultaneously, but that requires synchronization and we
    // don't need the throughput.
    //
    WDF_IO_QUEUE_CONFIG queueConfig;
    WDF_IO_QUEUE_CONFIG_INIT_DEFAULT_QUEUE(
        &queueConfig,
        WdfIoQueueDispatchSequential
    );
    queueConfig.EvtIoDeviceControl = KdEvtIoDeviceControl;

    WDFQUEUE queue;
    status = WdfIoQueueCreate(
        device,
        &queueConfig,
        WDF_NO_OBJECT_ATTRIBUTES,
        &queue
    );
    if (!NT_SUCCESS(status)) {
        KdPrint(("kd-input: WdfIoQueueCreate failed: 0x%x\n", status));
        return status;
    }

    return STATUS_SUCCESS;
}

//
// KdEvtDevicePrepareHardware
//
// Called when our device is powered on and ready to use.
// For a virtual device like ours, this happens right after device creation.
//
// This is where we create the VHF virtual HID device. After this returns
// successfully, Windows sees our virtual gamepad/keyboard/mouse in
// Device Manager and games can find it via XInput/DirectInput/RawInput.
//
// Parameters:
//   Device              — our WDF device object
//   ResourcesRaw       — hardware resources (IRQs, memory ranges) assigned
//   ResourcesTranslated — translated versions of the above
//   Both are empty for our virtual device — we have no real hardware.
//
NTSTATUS KdEvtDevicePrepareHardware(
    _In_ WDFDEVICE    Device,
    _In_ WDFCMRESLIST ResourcesRaw,
    _In_ WDFCMRESLIST ResourcesTranslated
)
{
    UNREFERENCED_PARAMETER(ResourcesRaw);
    UNREFERENCED_PARAMETER(ResourcesTranslated);

    NTSTATUS status = KdCreateVhfDevice(Device);
    if (!NT_SUCCESS(status)) {
        KdPrint(("kd-input: KdCreateVhfDevice failed: 0x%x\n", status));
    }
    return status;
}

//
// KdEvtDeviceReleaseHardware
//
// Called when our device is being powered down or removed.
// We destroy the VHF device here, which removes our virtual HID device
// from Windows. Games will no longer see it after this returns.
//
NTSTATUS KdEvtDeviceReleaseHardware(
    _In_ WDFDEVICE    Device,
    _In_ WDFCMRESLIST ResourcesTranslated
)
{
    UNREFERENCED_PARAMETER(Device);
    UNREFERENCED_PARAMETER(ResourcesTranslated);

    KdDestroyVhfDevice();
    return STATUS_SUCCESS;
}