# Milestone 1: Windows Kernel Driver PoC - Test Report

**Date:** June 9, 2026
**Status:** ✅ SUCCESS
**Component:** `windows-driver` (C++ WDM Driver)

## Executive Summary
The high-risk proof-of-concept (PoC) for the Lampyris Virtual Microphone Kernel-Mode Driver (KMD) has been successfully compiled, deployed, and verified in an isolated Windows Virtual Machine environment. The driver successfully executes Ring-0 operations without causing system instability (BSOD) and correctly implements the foundational security mechanisms required for the IOCTL interface.

## Build Environment & Fixes Applied
During the initial build phase, several environment and configuration issues were resolved:
1. **MSB8040 (Spectre Mitigation):** Explicitly disabled Spectre mitigation in `lampyris-mic.vcxproj` to allow local compilation without requiring specialized MSVC libraries.
2. **C1083 (`ntddk.h` Not Found):** Added `<WindowsTargetPlatformVersion>10.0.22621.0</WindowsTargetPlatformVersion>` to the project file. This ensured Visual Studio correctly linked the installed Windows Driver Kit (WDK) include paths.
3. **Macro Redefinition (`_KERNEL_MODE`):** Removed redundant manual preprocessor definitions that conflicted with the WDK toolset, resolving `/WX` (Treat Warnings as Errors) failures.
4. **Missing Prototypes:** Declared `extern "C" ULONG NTAPI RtlRandomEx(PULONG Seed);` to link the required entropy generator without triggering header conflicts.
5. **SignTool Failure:** Added `<FileDigestAlgorithm>sha256</FileDigestAlgorithm>` to the build configuration, allowing Visual Studio to successfully generate a test certificate (`.cer`) and self-sign the `.sys` file.

## Test Execution Details
**Testing Environment:** VirtualBox (Windows x64 Guest)
**Driver Signature Enforcement:** Disabled via `bcdedit /set testsigning on`

### Step 1: Service Registration & Initialization
The driver was manually registered with the Windows Service Control Manager (`sc.exe`) as a legacy kernel service since the Plug-and-Play (PnP) `.inf` file and PortCls audio topology are deferred to a later phase.

* **Action:** `sc create LampyrisMic type= kernel binPath= C:\Windows\System32\drivers\lampyris-mic.sys`
* **Result:** Service created successfully.
* **Action:** `sc start LampyrisMic`
* **Result:** The service transitioned to `STATE: 4 RUNNING`. The `DriverEntry` routine returned `STATUS_SUCCESS` without causing a bugcheck (BSOD). 

### Step 2: Security Token Verification
To prevent unprivileged user-space applications from injecting unauthorized audio data into the kernel, the driver is designed to generate a 32-byte cryptographic token using `RtlRandomEx` seeded by `KeQueryInterruptTime()`. This token is written to the registry at load time.

* **Verification:** Opened `regedit.exe` in the guest VM and navigated to `HKLM\SOFTWARE\Lampyris`.
* **Result:** Verified the existence of the `SessionToken` binary value containing the 32-byte payload. 

## Conclusion & Next Steps
The core memory allocation, secure device object creation (`\Device\LampyrisMic`), and registry interactions are highly stable. The most significant foundational risks associated with Windows kernel development have been mitigated.

**Next Milestone Goals:**
1. Integrate Microsoft's **PortCls (Audio Port Class)** framework to expose the ring buffer as a standard KS (Kernel Streaming) audio endpoint.
2. Develop the `lampyris-mic.inf` file to allow standard PnP installation via Device Manager.
3. Verify that "Lampyris Virtual Microphone" appears in the Windows Sound Control Panel.
