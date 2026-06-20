# Diagnosis: The "Silent Stream" Bug

After successfully defeating the Fake Peak Meter and fixing the format rejection, the audio stream is finally starting correctly. The Rust Client connects, receives the audio from the mobile client, and successfully updates its internal visualizer. However, the Windows driver produces absolutely no sound. 

## The Root Cause: IOCTL Payload Size Mismatch

The culprit is a structural definition mismatch between the Rust Client and the Windows Driver regarding the **IOCTL Push Payload size**.

1. **The Rust Client Changes:** In our previous fix to handle Stereo audio, we doubled the `LAMPYRIS_MAX_AUDIO_PAYLOAD` in `pc-client/src/audio/windows.rs` from `4800` bytes to `9600` bytes. The Rust client is now successfully packaging and sending 9600-byte payloads to the kernel.
2. **The Driver Blindspot:** The driver's shared header file, `windows-driver/ioctl.h`, **still defines** `LAMPYRIS_MAX_AUDIO_PAYLOAD` as `4800` bytes.
3. **The Silent Drop:** When the driver receives the IOCTL push command in `lampyris_core.cpp`, it performs a boundary check:
   ```cpp
   if (Audio->Length > LAMPYRIS_MAX_AUDIO_PAYLOAD) { // 9600 > 4800
       return STATUS_INVALID_BUFFER_SIZE;
   }
   ```
   Because `9600 > 4800` is `true`, the driver quietly rejects and drops **every single audio packet** sent by the PC Client. 
4. **The Consequence:** Because no data is ever written to the driver's internal `g_AudioRingBuffer`, it remains filled with zeroes. When the Windows Audio Engine periodically pulls data via WaveRT DMA (`ReadAudioData`), the driver feeds it pure silence, resulting in no sound output.

## Recommended Fix

We need to synchronize the IOCTL headers. We must update `windows-driver/ioctl.h` to increase `LAMPYRIS_MAX_AUDIO_PAYLOAD` to `9600` so the driver accepts the larger stereo packets. Once changed, we will re-compile the Windows driver and reinstall it.