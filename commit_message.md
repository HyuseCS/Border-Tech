# Commit Summary

**Title**: Fix driver IOCTL status, optimize rust client audio loop, and configure static MSVC CRT 

**Description**:
*   **Driver Kernel Patch (`lampyris_core.cpp`)**: 
    Fixed an issue where `DeviceIoControl` would fail in user space due to the Windows IO Manager overriding the completion status with `STATUS_BUFFER_OVERFLOW` (0x80000005). The `BytesTransferred` is now correctly set to `0` instead of `Audio->Length` when processing `IOCTL_LAMPYRIS_PUSH_AUDIO` because there is no output buffer expected.
*   **Audio Engine Jitter Fix (`pc-client/src/audio/windows.rs`)**: 
    Re-wrote the core `run_loop` to completely remove an unconditional `std::thread::sleep(5ms)`. Previously, default Windows timer resolutions caused this sleep to extend up to ~15.6ms, leading to severe buffer starvation in the driver every cycle. 
*   **Resiliency Buffering (`pc-client/src/audio/windows.rs`)**: 
    Increased the `prebuffer_threshold` from 10ms to 50ms, allowing the ring buffer to absorb packet arrival jitter without underrunning the Kernel driver.
*   **Runtime Dependency (`pc-client/.cargo/config.toml`)**: 
    Configured the PC client to compile the `x86_64-pc-windows-msvc` target with `target-feature=+crt-static`, eliminating the dynamic CRT dependency on `VCRUNTIME140.dll` and making it portable to bare-bones VMs out of the box.

---

### Known Issues & Current State
**Status: Audio Output Still Silent**
While the end-to-end connection works flawlessly (Android -> PC Client -> Driver IOCTL), the virtual microphone device is still not producing audio in Windows. 
1.  **Pipeline Verified**: PC client successfully detects mobile audio (UI visualizer moves).
2.  **Driver I/O Verified**: `lampyris.exe` pushes `LampyrisAudioPayload` to the driver successfully.
3.  **Failure Point**: The `Listen to this device` option yields no sound, and the Windows Sound Control Panel meter does not move. The data entering `g_AudioRingBuffer` is seemingly not being correctly picked up by the `CMiniportWaveRT` audio engine hooks, or the audio format properties do not match what Windows is expecting to render.
