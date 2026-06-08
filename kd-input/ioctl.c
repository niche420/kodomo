#include <ntddk.h>
#include <wdf.h>
#include "public.h"

//
// Forward declarations from report.c
//
NTSTATUS KdSubmitGamepadReport(KD_GAMEPAD_REPORT* report);
NTSTATUS KdSubmitKeyboardReport(KD_KEYBOARD_REPORT* report);
NTSTATUS KdSubmitMouseReport(KD_MOUSE_REPORT* report);

//
// KdEvtIoDeviceControl
//
// This is called by KMDF whenever kd-server calls DeviceIoControl().
// It runs in an arbitrary thread context at IRQL PASSIVE_LEVEL, meaning
// we can safely access pageable memory and block — though we don't block.
//
// Parameters:
//   Queue              — the I/O queue this request came from
//   Request            — the WDF request object wrapping the IRP
//   OutputBufferLength — size of the output buffer kd-server provided
//                        (we don't send anything back, so this is 0)
//   InputBufferLength  — size of the input buffer kd-server sent
//   IoControlCode      — which IOCTL was called (our defined codes)
//
// For METHOD_BUFFERED IOCTLs (which is what we use), Windows has already
// copied kd-server's input buffer into a kernel buffer for us. We get a
// pointer to that kernel buffer via WdfRequestRetrieveInputBuffer.
// We never touch userspace memory directly — the OS handles the copy.
//
VOID KdEvtIoDeviceControl(
    _In_ WDFQUEUE   Queue,
    _In_ WDFREQUEST Request,
    _In_ size_t     OutputBufferLength,
    _In_ size_t     InputBufferLength,
    _In_ ULONG      IoControlCode
)
{
    UNREFERENCED_PARAMETER(Queue);
    UNREFERENCED_PARAMETER(OutputBufferLength);

    NTSTATUS status;
    PVOID    buffer;
    size_t   bufferSize;

    //
    // Retrieve the input buffer.
    //
    // WdfRequestRetrieveInputBuffer gives us a pointer to the kernel copy
    // of the data kd-server passed to DeviceIoControl. The third parameter
    // is the minimum size we require — if the buffer is smaller than this,
    // WDF returns an error before we even look at it.
    //
    // We pass 1 as the minimum here and do our own size checks below,
    // because the minimum size depends on which IOCTL was called.
    //
    status = WdfRequestRetrieveInputBuffer(
        Request,
        1,
        &buffer,
        &bufferSize
    );
    if (!NT_SUCCESS(status)) {
        KdPrint(("kd-input: WdfRequestRetrieveInputBuffer failed: 0x%x\n", status));
        WdfRequestComplete(Request, status);
        return;
    }

    //
    // Dispatch on the IOCTL code.
    //
    // For each case we:
    //   1. Verify the buffer is exactly the right size for the report struct.
    //      This is important for security — we must not read past the end
    //      of the buffer the caller provided.
    //   2. Cast the buffer pointer to the appropriate report struct.
    //   3. Call the corresponding submit function in report.c.
    //
    switch (IoControlCode)
    {
    case IOCTL_KD_GAMEPAD_REPORT:

        if (InputBufferLength < sizeof(KD_GAMEPAD_REPORT)) {
            KdPrint(("kd-input: gamepad buffer too small: %zu < %zu\n",
                InputBufferLength, sizeof(KD_GAMEPAD_REPORT)));
            status = STATUS_BUFFER_TOO_SMALL;
            break;
        }

        status = KdSubmitGamepadReport((KD_GAMEPAD_REPORT*)buffer);
        break;

    case IOCTL_KD_KEYBOARD_REPORT:

        if (InputBufferLength < sizeof(KD_KEYBOARD_REPORT)) {
            KdPrint(("kd-input: keyboard buffer too small: %zu < %zu\n",
                InputBufferLength, sizeof(KD_KEYBOARD_REPORT)));
            status = STATUS_BUFFER_TOO_SMALL;
            break;
        }

        status = KdSubmitKeyboardReport((KD_KEYBOARD_REPORT*)buffer);
        break;

    case IOCTL_KD_MOUSE_REPORT:

        if (InputBufferLength < sizeof(KD_MOUSE_REPORT)) {
            KdPrint(("kd-input: mouse buffer too small: %zu < %zu\n",
                InputBufferLength, sizeof(KD_MOUSE_REPORT)));
            status = STATUS_BUFFER_TOO_SMALL;
            break;
        }

        status = KdSubmitMouseReport((KD_MOUSE_REPORT*)buffer);
        break;

    default:
        //
        // Unknown IOCTL code. This happens if kd-server sends a code we
        // don't recognize — either a bug or a version mismatch.
        //
        KdPrint(("kd-input: unknown IOCTL: 0x%x\n", IoControlCode));
        status = STATUS_INVALID_DEVICE_REQUEST;
        break;
    }

    //
    // Complete the request.
    //
    // Every WDF request MUST be completed exactly once. If we forget to
    // call WdfRequestComplete, kd-server's DeviceIoControl call will hang
    // forever waiting for a response. If we call it twice, the system
    // crashes.
    //
    // The second parameter is the status code — STATUS_SUCCESS tells
    // kd-server the IOCTL succeeded. Any error code causes
    // DeviceIoControl to return FALSE and sets the last error.
    //
    // The third parameter (0) is the number of bytes written to the output
    // buffer. We don't write any output, so it's 0.
    //
    WdfRequestCompleteWithInformation(Request, status, 0);
}