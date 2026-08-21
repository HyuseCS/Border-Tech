#include <ntddk.h>
#include <ntstrsafe.h>
#include <wdmsec.h>
#include "ioctl.h"

// Global driver state
static PDEVICE_OBJECT g_DeviceObject = NULL;

// Audio ring buffer configuration: 2 seconds of 48kHz mono 16-bit PCM (192,000 bytes)
#define RING_BUFFER_SIZE (48000 * 2 * 2)
static UCHAR* g_AudioRingBuffer = NULL;
static ULONG g_RingBufferWriteOffset = 0;
static ULONG g_RingBufferReadOffset = 0;
static ULONG g_RingBufferLength = 0;
static KSPIN_LOCK g_BufferLock;

// Function declarations
extern "C" {
    DRIVER_INITIALIZE DriverEntry;
    DRIVER_UNLOAD LampyrisUnload;
    
    NTSTATUS DriverEntry(PDRIVER_OBJECT DriverObject, PUNICODE_STRING RegistryPath);
    VOID LampyrisUnload(PDRIVER_OBJECT DriverObject);
}

NTSTATUS LampyrisDefaultDispatch(PDEVICE_OBJECT DeviceObject, PIRP Irp);
NTSTATUS LampyrisCreateClose(PDEVICE_OBJECT DeviceObject, PIRP Irp);
NTSTATUS LampyrisDeviceControl(PDEVICE_OBJECT DeviceObject, PIRP Irp);
NTSTATUS PushAudioData(PVOID Buffer, ULONG Length);

// Push audio data into the circular buffer. Overwrites oldest data on overflow.
NTSTATUS PushAudioData(PVOID Buffer, ULONG Length) {
    KLOCK_QUEUE_HANDLE LockHandle;
    
    if (Length > LAMPYRIS_MAX_AUDIO_PAYLOAD) {
        return STATUS_INVALID_BUFFER_SIZE;
    }
    
    if (g_AudioRingBuffer == NULL) {
        return STATUS_DEVICE_NOT_READY;
    }
    
    KeAcquireInStackQueuedSpinLock(&g_BufferLock, &LockHandle);
    
    for (ULONG i = 0; i < Length; i++) {
        g_AudioRingBuffer[g_RingBufferWriteOffset] = ((UCHAR*)Buffer)[i];
        g_RingBufferWriteOffset = (g_RingBufferWriteOffset + 1) % RING_BUFFER_SIZE;
        
        if (g_RingBufferLength < RING_BUFFER_SIZE) {
            g_RingBufferLength++;
        } else {
            // Buffer full: silently drop the oldest byte to avoid blocking
            g_RingBufferReadOffset = (g_RingBufferReadOffset + 1) % RING_BUFFER_SIZE;
        }
    }
    
    KeReleaseInStackQueuedSpinLock(&LockHandle);
    return STATUS_SUCCESS;
}

// Reads audio data from the circular buffer. Used by the PortCls miniport stream.
// Fills the remainder of the buffer with silence (0) if there's underrun.
ULONG ReadAudioData(PVOID Buffer, ULONG Length) {
    KLOCK_QUEUE_HANDLE LockHandle;
    ULONG BytesRead = 0;
    
    if (g_AudioRingBuffer == NULL) {
        return 0;
    }
    
    KeAcquireInStackQueuedSpinLock(&g_BufferLock, &LockHandle);
    
    ULONG Available = g_RingBufferLength;
    ULONG ToCopy = (Length < Available) ? Length : Available;
    
    for (ULONG i = 0; i < ToCopy; i++) {
        ((UCHAR*)Buffer)[i] = g_AudioRingBuffer[g_RingBufferReadOffset];
        g_RingBufferReadOffset = (g_RingBufferReadOffset + 1) % RING_BUFFER_SIZE;
        g_RingBufferLength--;
        BytesRead++;
    }
    
    if (BytesRead < Length) {
        RtlZeroMemory((UCHAR*)Buffer + BytesRead, Length - BytesRead);
    }
    
    KeReleaseInStackQueuedSpinLock(&LockHandle);
    return BytesRead;
}

// Driver entry point
extern "C" NTSTATUS DriverEntry(
    PDRIVER_OBJECT DriverObject,
    PUNICODE_STRING RegistryPath
) {
    UNREFERENCED_PARAMETER(RegistryPath);
    
    NTSTATUS Status;
    UNICODE_STRING DeviceName;
    UNICODE_STRING SymbolicLinkName;
    
    RtlInitUnicodeString(&DeviceName, L"\\Device\\LampyrisMic");
    RtlInitUnicodeString(&SymbolicLinkName, L"\\DosDevices\\LampyrisMic");
    
    KeInitializeSpinLock(&g_BufferLock);
    
    // Allocate non-paged memory for ring buffer (secure pool allocation)
    g_AudioRingBuffer = (UCHAR*)ExAllocatePool2(POOL_FLAG_NON_PAGED, RING_BUFFER_SIZE, 'LMPY');
    if (g_AudioRingBuffer == NULL) {
        return STATUS_INSUFFICIENT_RESOURCES;
    }
    
    // Secure Device Creation: System and Interactive User get full access
    DECLARE_CONST_UNICODE_STRING(SddlString, L"D:P(A;;GA;;;SY)(A;;GA;;;IU)");
    
    Status = IoCreateDeviceSecure(
        DriverObject,
        0,
        &DeviceName,
        FILE_DEVICE_LAMPYRIS,
        FILE_DEVICE_SECURE_OPEN,
        FALSE,
        &SddlString,
        NULL,
        &g_DeviceObject
    );
    
    if (!NT_SUCCESS(Status)) {
        ExFreePoolWithTag(g_AudioRingBuffer, 'LMPY');
        g_AudioRingBuffer = NULL;
        return Status;
    }
    
    // Register major functions
    for (ULONG i = 0; i <= IRP_MJ_MAXIMUM_FUNCTION; i++) {
        DriverObject->MajorFunction[i] = LampyrisDefaultDispatch;
    }
    
    DriverObject->MajorFunction[IRP_MJ_CREATE] = LampyrisCreateClose;
    DriverObject->MajorFunction[IRP_MJ_CLOSE] = LampyrisCreateClose;
    DriverObject->MajorFunction[IRP_MJ_DEVICE_CONTROL] = LampyrisDeviceControl;
    DriverObject->DriverUnload = LampyrisUnload;
    
    Status = IoCreateSymbolicLink(&SymbolicLinkName, &DeviceName);
    if (!NT_SUCCESS(Status)) {
        IoDeleteDevice(g_DeviceObject);
        g_DeviceObject = NULL;
        ExFreePoolWithTag(g_AudioRingBuffer, 'LMPY');
        g_AudioRingBuffer = NULL;
        return Status;
    }
    
    return STATUS_SUCCESS;
}

// Unload routine
VOID LampyrisUnload(PDRIVER_OBJECT DriverObject) {
    UNREFERENCED_PARAMETER(DriverObject);
    
    UNICODE_STRING SymbolicLinkName;
    RtlInitUnicodeString(&SymbolicLinkName, L"\\DosDevices\\LampyrisMic");
    
    IoDeleteSymbolicLink(&SymbolicLinkName);
    
    if (g_DeviceObject != NULL) {
        IoDeleteDevice(g_DeviceObject);
        g_DeviceObject = NULL;
    }
    
    if (g_AudioRingBuffer != NULL) {
        ExFreePoolWithTag(g_AudioRingBuffer, 'LMPY');
        g_AudioRingBuffer = NULL;
    }
}

// Default IRP dispatcher
NTSTATUS LampyrisDefaultDispatch(PDEVICE_OBJECT DeviceObject, PIRP Irp) {
    UNREFERENCED_PARAMETER(DeviceObject);
    Irp->IoStatus.Status = STATUS_SUCCESS;
    Irp->IoStatus.Information = 0;
    IoCompleteRequest(Irp, IO_NO_INCREMENT);
    return STATUS_SUCCESS;
}

// Create/Close handler
NTSTATUS LampyrisCreateClose(PDEVICE_OBJECT DeviceObject, PIRP Irp) {
    UNREFERENCED_PARAMETER(DeviceObject);
    
    Irp->IoStatus.Status = STATUS_SUCCESS;
    Irp->IoStatus.Information = 0;
    IoCompleteRequest(Irp, IO_NO_INCREMENT);
    return STATUS_SUCCESS;
}

// Device Control dispatcher
NTSTATUS LampyrisDeviceControl(PDEVICE_OBJECT DeviceObject, PIRP Irp) {
    UNREFERENCED_PARAMETER(DeviceObject);
    
    PIO_STACK_LOCATION IrpSp = IoGetCurrentIrpStackLocation(Irp);
    NTSTATUS Status = STATUS_INVALID_DEVICE_REQUEST;
    ULONG BytesTransferred = 0;
    
    ULONG IoControlCode = IrpSp->Parameters.DeviceIoControl.IoControlCode;
    ULONG InputBufferLength = IrpSp->Parameters.DeviceIoControl.InputBufferLength;
    PVOID SystemBuffer = Irp->AssociatedIrp.SystemBuffer;
    
    switch (IoControlCode) {
        case IOCTL_LAMPYRIS_PUSH_AUDIO: {
            if (InputBufferLength < sizeof(LAMPYRIS_AUDIO_PAYLOAD)) {
                Status = STATUS_INVALID_PARAMETER;
                break;
            }
            
            PLAMPYRIS_AUDIO_PAYLOAD Audio = (PLAMPYRIS_AUDIO_PAYLOAD)SystemBuffer;
            
            if (Audio->Length > LAMPYRIS_MAX_AUDIO_PAYLOAD) {
                Status = STATUS_INVALID_BUFFER_SIZE;
                break;
            }
            
            if (InputBufferLength < FIELD_OFFSET(LAMPYRIS_AUDIO_PAYLOAD, Data) + Audio->Length) {
                Status = STATUS_INVALID_PARAMETER;
                break;
            }
            
            Status = PushAudioData(Audio->Data, Audio->Length);
            if (NT_SUCCESS(Status)) {
                BytesTransferred = Audio->Length;
            }
            break;
        }
        
        default:
            Status = STATUS_INVALID_DEVICE_REQUEST;
            break;
    }
    
    Irp->IoStatus.Status = Status;
    Irp->IoStatus.Information = BytesTransferred;
    IoCompleteRequest(Irp, IO_NO_INCREMENT);
    return Status;
}
