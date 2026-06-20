# Commit Summary

**Title**: Implement rate-limited DbgPrint kernel tracing for audio engine diagnostics

**Description**:
*   **Safety-First Instrumentation**: Injected targeted, rate-limited `DbgPrint` statements into the `lampyris-sysvad` Windows driver to safely trace the audio execution flow without freezing the DPC queue or causing watchdog BSODs.
*   **IOCTL & Ring Buffer Tracing**: Added logging to `LampyrisDeviceControl` (in `lampyris_core.cpp`) to verify the exact payload size being pushed by the PC client and the current available capacity of `g_AudioRingBuffer`.
*   **Data Copy Tracing**: Added logging to `ReadAudioData` (in `lampyris_core.cpp`) to track exactly how many bytes the Windows Audio Engine requests versus how many bytes the driver successfully copies out of the ring buffer.
*   **Audio Engine Hooks**: Instrumented `CMiniportWaveRTStream::GetPosition`, `UpdatePosition`, and `TimerNotifyRT` (in `minwavertstream.cpp`) to monitor the hardware DMA cursor (`PlayOffset` / `WriteOffset`), calculated byte displacement, and the firing of event-driven audio notifications.
*   **Driver Compilation**: Rebuilt and test-signed the kernel driver (`lampyris-mic.sys`) for x64 Release using MSBuild.

### Next Steps / Context
This commit leaves the driver fully instrumented for DebugView diagnosis. The goal is to identify why "Listen to this device" yields silent output, specifically checking if the OS rejects the format (no `GetPosition` calls), if the DMA clock is frozen, if the event timer is starving, or if it's a simple buffer under-run issue.
