# Roadmap: Lampyris Driver Audio Pipeline Fixes

This is our master checklist for resolving the format rejection (dead stream), the fake volume meter, and the compilation blindspot.

## Step 1: Fix the Compilation Blindspot
**Context:** Modifications to `minwavertstream.cpp` (like our `RtlZeroMemory` and `DbgPrint` injections) did not compile because `EndpointsCommon.vcxproj` is missing from the solution.
- [ ] **Action:** Add `EndpointsCommon\EndpointsCommon.vcxproj` to `lampyris-mic.sln`.
- [ ] **Action:** Ensure it is included in the build configuration for `Release|x64`.

## Step 2: Fix the Fake Peak Meter
**Context:** Sysvad hardcodes a peak meter to 50% on initialization. This causes the Sound Control Panel to display a frozen volume meter even when the stream is stopped.
- [ ] **Action:** Open `lampyris-sysvad\hw.cpp`.
- [ ] **Action:** Change `m_PeakMeterControls[i] = PEAKMETER_SIGNED_MAXIMUM/2;` (Line ~345) to `m_PeakMeterControls[i] = 0;`.

## Step 3: Resolve Format Rejection in Kernel
**Context:** Windows Audio Engine demands Stereo (2-channel) endpoints. The driver advertises Mono (1-channel), causing the stream to be rejected silently.
- [ ] **Action:** Open `lampyris-sysvad\TabletAudioSample\micinwavtable.h`.
- [ ] **Action:** Change `MICIN_DEVICE_MAX_CHANNELS` from `1` to `2`.
- [ ] **Action:** Locate the 48000Hz format block (index 7). Change the channels from `1` to `2` and the speaker configuration from `KSAUDIO_SPEAKER_MONO` to `KSAUDIO_SPEAKER_STEREO`.

## Step 4: Resolve Format Rejection in Rust Client
**Context:** Because the kernel driver will now expect Stereo, the Rust client must double the Mono samples coming from Android into Stereo before pushing to the IOCTL.
- [ ] **Action:** Open `pc-client\src\protocol.rs` (or `windows.rs`).
- [ ] **Action:** Modify the payload handler to duplicate channels for 48kHz audio (expand 2400 Mono samples to 4800 Stereo samples).

## Step 5: Rebuild & Verify
- [ ] **Action:** Re-run MSBuild to compile `lampyris-mic.sln` and ensure `EndpointsCommon.lib` is rebuilt.
- [ ] **Action:** Re-run `cargo build --release` for the PC client.
- [ ] **Action:** Inform the user to install the signed driver and test.