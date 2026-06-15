#pragma once

#ifdef _KERNEL_MODE
#include <ntddk.h>
#include <windef.h>
#else
#include <windows.h>
#include <winioctl.h>
#endif

// Custom device type (in user-defined range 32768-65535)
#define FILE_DEVICE_LAMPYRIS 0x8001

// Function codes (in user-defined range 2048-4095)
#define LAMPYRIS_FUNC_PUSH_AUDIO    0x802

// IOCTL Definitions using METHOD_BUFFERED and FILE_WRITE_ACCESS
#define IOCTL_LAMPYRIS_PUSH_AUDIO \
    CTL_CODE(FILE_DEVICE_LAMPYRIS, LAMPYRIS_FUNC_PUSH_AUDIO, METHOD_BUFFERED, FILE_WRITE_ACCESS)

// Audio payload structure (max payload size 4800 bytes)
#define LAMPYRIS_MAX_AUDIO_PAYLOAD 4800

typedef struct _LAMPYRIS_AUDIO_PAYLOAD {
    ULONG Length; // Length of raw PCM data
    BYTE Data[LAMPYRIS_MAX_AUDIO_PAYLOAD];
} LAMPYRIS_AUDIO_PAYLOAD, *PLAMPYRIS_AUDIO_PAYLOAD;

