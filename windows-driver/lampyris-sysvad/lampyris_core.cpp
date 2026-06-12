#include <ntddk.h>
#include <ntstrsafe.h>
#include <wdmsec.h>
#include "..\ioctl.h"

#ifdef ExAllocatePool2
#undef ExAllocatePool2
#endif
#pragma warning(disable: 4996)
#pragma warning(disable: 4100)
__inline PVOID LampyrisExAllocatePool2Core(ULONG64 Flags, SIZE_T Size, ULONG Tag) { PVOID p = ExAllocatePoolWithTag(NonPagedPoolNx, Size, Tag); if (p) { RtlZeroMemory(p, Size); } return p; }
#define ExAllocatePool2 LampyrisExAllocatePool2Core

// Global driver state
UCHAR g_SessionToken[32];
BOOLEAN g_Authenticated = FALSE;
PDEVICE_OBJECT g_ControlDeviceObject = NULL;

// Audio ring buffer configuration: 2 seconds of 48kHz mono 16-bit PCM (192,000 bytes)
#define RING_BUFFER_SIZE (48000 * 2 * 2)
UCHAR* g_AudioRingBuffer = NULL;
ULONG g_RingBufferWriteOffset = 0;
ULONG g_RingBufferReadOffset = 0;
ULONG g_RingBufferLength = 0;
KSPIN_LOCK g_BufferLock;

PDRIVER_DISPATCH g_PcDeviceControl = NULL;
PDRIVER_DISPATCH g_PcCreate = NULL;
PDRIVER_DISPATCH g_PcClose = NULL;

extern "C" ULONG NTAPI RtlRandomEx(PULONG Seed);

VOID GenerateRandomToken(UCHAR* Buffer, ULONG Length) {
    ULONG Seed = (ULONG)KeQueryInterruptTime();
    for (ULONG i = 0; i < Length; i++) {
        Buffer[i] = (UCHAR)(RtlRandomEx(&Seed) & 0xFF);
    }
}

NTSTATUS WriteTokenToRegistry(VOID) {
    NTSTATUS Status = RtlWriteRegistryValue(
        RTL_REGISTRY_ABSOLUTE,
        L"\\Registry\\Machine\\Software\\Lampyris",
        L"SessionToken",
        REG_BINARY,
        g_SessionToken,
        32
    );
    return Status;
}

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
            g_RingBufferReadOffset = (g_RingBufferReadOffset + 1) % RING_BUFFER_SIZE;
        }
    }
    
    KeReleaseInStackQueuedSpinLock(&LockHandle);
    return STATUS_SUCCESS;
}

ULONG ReadAudioData(PVOID Buffer, ULONG Length) {
    KLOCK_QUEUE_HANDLE LockHandle;
    ULONG BytesRead = 0;
    
    if (g_AudioRingBuffer == NULL) {
        RtlZeroMemory(Buffer, Length);
        return Length;
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
    return Length;
}

NTSTATUS LampyrisCreateClose(PDEVICE_OBJECT DeviceObject, PIRP Irp) {
    if (DeviceObject != g_ControlDeviceObject) {
        PIO_STACK_LOCATION IrpSp = IoGetCurrentIrpStackLocation(Irp);
        if (IrpSp->MajorFunction == IRP_MJ_CREATE && g_PcCreate) return g_PcCreate(DeviceObject, Irp);
        if (IrpSp->MajorFunction == IRP_MJ_CLOSE && g_PcClose) return g_PcClose(DeviceObject, Irp);
        Irp->IoStatus.Status = STATUS_SUCCESS;
        Irp->IoStatus.Information = 0;
        IoCompleteRequest(Irp, IO_NO_INCREMENT);
        return STATUS_SUCCESS;
    }
    
    PIO_STACK_LOCATION IrpSp = IoGetCurrentIrpStackLocation(Irp);
    if (IrpSp->MajorFunction == IRP_MJ_CREATE) {
        g_Authenticated = FALSE;
    }
    
    Irp->IoStatus.Status = STATUS_SUCCESS;
    Irp->IoStatus.Information = 0;
    IoCompleteRequest(Irp, IO_NO_INCREMENT);
    return STATUS_SUCCESS;
}

NTSTATUS LampyrisDeviceControl(PDEVICE_OBJECT DeviceObject, PIRP Irp) {
    if (DeviceObject != g_ControlDeviceObject) {
        if (g_PcDeviceControl) return g_PcDeviceControl(DeviceObject, Irp);
        Irp->IoStatus.Status = STATUS_INVALID_DEVICE_REQUEST;
        Irp->IoStatus.Information = 0;
        IoCompleteRequest(Irp, IO_NO_INCREMENT);
        return STATUS_INVALID_DEVICE_REQUEST;
    }
    
    PIO_STACK_LOCATION IrpSp = IoGetCurrentIrpStackLocation(Irp);
    NTSTATUS Status = STATUS_INVALID_DEVICE_REQUEST;
    ULONG BytesTransferred = 0;
    
    ULONG IoControlCode = IrpSp->Parameters.DeviceIoControl.IoControlCode;
    ULONG InputBufferLength = IrpSp->Parameters.DeviceIoControl.InputBufferLength;
    PVOID SystemBuffer = Irp->AssociatedIrp.SystemBuffer;
    
    switch (IoControlCode) {
        case IOCTL_LAMPYRIS_AUTHENTICATE: {
            if (InputBufferLength < sizeof(LAMPYRIS_AUTH_PAYLOAD)) {
                Status = STATUS_INVALID_PARAMETER;
                break;
            }
            
            PLAMPYRIS_AUTH_PAYLOAD Auth = (PLAMPYRIS_AUTH_PAYLOAD)SystemBuffer;
            BOOLEAN Match = TRUE;
            for (int i = 0; i < 32; i++) {
                if (Auth->Token[i] != g_SessionToken[i]) Match = FALSE;
            }
            
            if (Match) {
                g_Authenticated = TRUE;
                Status = STATUS_SUCCESS;
            } else {
                g_Authenticated = FALSE;
                Status = STATUS_ACCESS_DENIED;
            }
            break;
        }
        
        case IOCTL_LAMPYRIS_PUSH_AUDIO: {
            if (!g_Authenticated) {
                Status = STATUS_ACCESS_DENIED;
                break;
            }
            
            if (InputBufferLength < sizeof(LAMPYRIS_AUDIO_PAYLOAD)) {
                Status = STATUS_INVALID_PARAMETER;
                break;
            }
            
            PLAMPYRIS_AUDIO_PAYLOAD Audio = (PLAMPYRIS_AUDIO_PAYLOAD)SystemBuffer;
            
            if (Audio->Length > LAMPYRIS_MAX_AUDIO_PAYLOAD) {
                Status = STATUS_INVALID_BUFFER_SIZE;
                break;
            }
            
            Status = PushAudioData(Audio->Data, Audio->Length);
            if (NT_SUCCESS(Status)) BytesTransferred = Audio->Length;
            break;
        }
    }
    
    Irp->IoStatus.Status = Status;
    Irp->IoStatus.Information = BytesTransferred;
    IoCompleteRequest(Irp, IO_NO_INCREMENT);
    return Status;
}

extern "C" NTSTATUS LampyrisInit(PDRIVER_OBJECT DriverObject) {
    NTSTATUS Status;
    UNICODE_STRING DeviceName;
    UNICODE_STRING SymbolicLinkName;
    
    RtlInitUnicodeString(&DeviceName, L"\\Device\\LampyrisMic2");
    RtlInitUnicodeString(&SymbolicLinkName, L"\\DosDevices\\LampyrisMic2");
    
    KeInitializeSpinLock(&g_BufferLock);
    
    g_AudioRingBuffer = (UCHAR*)ExAllocatePool2(POOL_FLAG_NON_PAGED, RING_BUFFER_SIZE, 'LMPY');
    if (g_AudioRingBuffer == NULL) return STATUS_INSUFFICIENT_RESOURCES;
    
    GenerateRandomToken(g_SessionToken, 32);
    WriteTokenToRegistry();
    
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
        &g_ControlDeviceObject
    );
    
    if (!NT_SUCCESS(Status)) return Status;
    
    IoCreateSymbolicLink(&SymbolicLinkName, &DeviceName);
    
    // Hook the major functions so we intercept IRPs aimed at our control device.
    g_PcCreate = DriverObject->MajorFunction[IRP_MJ_CREATE];
    g_PcClose = DriverObject->MajorFunction[IRP_MJ_CLOSE];
    g_PcDeviceControl = DriverObject->MajorFunction[IRP_MJ_DEVICE_CONTROL];
    
    DriverObject->MajorFunction[IRP_MJ_CREATE] = LampyrisCreateClose;
    DriverObject->MajorFunction[IRP_MJ_CLOSE] = LampyrisCreateClose;
    DriverObject->MajorFunction[IRP_MJ_DEVICE_CONTROL] = LampyrisDeviceControl;
    
    return STATUS_SUCCESS;
}

extern "C" VOID LampyrisCleanup() {
    UNICODE_STRING SymbolicLinkName;
    RtlInitUnicodeString(&SymbolicLinkName, L"\\DosDevices\\LampyrisMic2");
    IoDeleteSymbolicLink(&SymbolicLinkName);
    
    if (g_ControlDeviceObject != NULL) {
        IoDeleteDevice(g_ControlDeviceObject);
        g_ControlDeviceObject = NULL;
    }
    if (g_AudioRingBuffer != NULL) {
        ExFreePoolWithTag(g_AudioRingBuffer, 'LMPY');
        g_AudioRingBuffer = NULL;
    }
}
