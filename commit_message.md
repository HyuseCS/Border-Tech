Fix audio pipeline format rejection and fake peak meter

This commit addresses several critical issues preventing the Windows Virtual Audio Driver from properly streaming audio from the Rust client:

- **Build Pipeline Fix:** Integrated `EndpointsCommon.vcxproj` into `lampyris-mic.sln` to ensure modifications to `minwavertstream.cpp` (such as `RtlZeroMemory` fixes) are correctly compiled and statically linked into the driver payload.
- **Fake Volume Meter Fix:** Changed the hardcoded PeakMeter initialization in `hw.cpp` from `PEAKMETER_SIGNED_MAXIMUM / 2` to `0`, successfully resolving the bug where the volume meter was permanently stuck at 50%.
- **Windows Format Rejection Fix (Kernel):** Updated `micinwavtable.h` to officially advertise `KSAUDIO_SPEAKER_STEREO` (2 Channels) at 48000Hz. This ensures the Windows Audio Engine no longer rejects the capture stream format.
- **Format Match (Rust Client):** Updated `pc-client/src/audio/windows.rs` to double the `LAMPYRIS_MAX_AUDIO_PAYLOAD` to `9600` bytes and modified the audio loop to duplicate incoming Mono samples into Stereo (Left/Right) channels to accurately match the driver's new expected format.

*Note: A known issue remains regarding an IOCTL buffer size mismatch (`STATUS_INVALID_BUFFER_SIZE`) in `ioctl.h` causing silent packets, which will be addressed in a follow-up commit.*
