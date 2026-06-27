# Commit Summary

**Title**: Fix payload size mismatch in Windows Driver

**Description**:
- **Identified Silent Rejection**: Discovered that the PC Client (since the Stereo update) has been sending 9600-byte payload packets, while the Windows Driver was stubbornly compiled with a 4800-byte structure limit. This caused the driver to silently reject all audio data with a `STATUS_INVALID_BUFFER_SIZE` error before even attempting to inject it into the ring buffer.
- **Fixed Size Mismatch**: Increased `LAMPYRIS_MAX_AUDIO_PAYLOAD` from 4800 to 9600 bytes inside `windows-driver/ioctl.h` and `windows-driver/TestApp.cpp` to perfectly match the PC Client's expected format. Recompiled both the driver and the PC client so they speak the same 9600-byte stereo language.

---

### Known Issues & Next Steps
**Status**: The connection between PC Client and Driver is repaired, but the Windows Audio Engine is still outputting silence (the green level indicator doesn't move).

**For Next Time**:
1. Investigate the driver's DMA buffer injection logic. Specifically, we need to determine exactly *why* the data successfully landing in `g_AudioRingBuffer` isn't making it into `m_pDmaBuffer` in a format that the OS recognizes.
2. Cross-reference the architecture against established working implementations (e.g., `github.com/VirtualDrivers/Virtual-Audio-Driver`) to see how they correctly handle WaveRT simulation, `UpdatePosition` callbacks, and virtual DMA timers.
