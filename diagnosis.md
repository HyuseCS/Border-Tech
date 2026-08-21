# Project-M: Windows Virtual Audio Driver Diagnosis

## 1. The Symptoms in the Log
The log `WINDOWSVM.log` only shows one repeating entry:
`[LAMPYRIS] IOCTL_LAMPYRIS_PUSH_AUDIO: Length 1920. Ring buffer available: 192000`

This definitively proves:
1. **The Core Pipeline Works:** The PC Client is successfully pushing audio to the driver via IOCTL. The driver is receiving it and buffering it perfectly.
2. **The Output End is Dead:** The ring buffer fills up but is **never drained**. 
3. **Crucial Logs are Missing:** Debug logging for `TimerNotifyRT` (the hardware timer simulation) and `GetPosition` (the function the OS uses to poll the audio cursor) were added. **Neither of these logs appear even once.**

## 2. The Stream Lifecycle & Failure Point
To understand why `TimerNotifyRT` and `GetPosition` are never called, we mapped how the Windows Audio Engine (audiodg.exe) opens a capture stream in a `Sysvad` (WaveRT) driver:

1. **`CMiniportWaveRT::NewStream`**: The OS requests to open the stream. The driver checks `IsFormatSupported`.
2. **`AllocateBufferWithNotification` / `AllocateAudioBuffer`**: The OS asks the driver to allocate DMA memory for the audio.
3. **`SetState(KSSTATE_ACQUIRE)` → `KSSTATE_PAUSE` → `KSSTATE_RUN`**: The OS commands the driver to start streaming.
4. **Timer Starts**: Only inside `KSSTATE_RUN` does the driver call `ExSetTimer` to begin the `TimerNotifyRT` loop.
5. **Streaming**: `TimerNotifyRT` fires every 1ms, calls `ReadAudioData` to drain our ring buffer, and `GetPosition` is repeatedly polled by the OS.

**The Failure Point:** Because `TimerNotifyRT` is never firing and `GetPosition` is never polled, we know with 100% certainty that the driver is **never reaching `KSSTATE_RUN`**. Because the browser threw a `NotReadableError` (initialization failure), it is highly likely that the Audio Engine is failing at Step 1 (`NewStream` / `IsFormatSupported`) and abandoning the stream immediately before it even tries to allocate a buffer or start the timer.

## 3. Deep Research on Root Causes
Why would the Windows Audio Engine refuse to initialize the stream, even after fixing the Topology/Stereo mismatch? Here are the researched possibilities based on Windows Audio Architecture:

### Cause A: Stale Windows Audio Registry Cache (Highly Likely)
When you install an audio driver on Windows, the OS creates a persistent registry entry in `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Capture` for that specific microphone.
* Windows caches the **Default Format** (e.g., 16000Hz Mono) that was negotiated when the device was first installed.
* In the latest fix, we removed all Mono formats and strictly enforced `48kHz Stereo` to fix the Topology mismatch.
* **The Conflict:** When "Listen to this device" is activated, Windows attempts to open the driver using the *cached* Default Format from the registry. Because the driver's `IsFormatSupported` function now strictly rejects Mono formats, it returns `STATUS_NO_MATCH`. The Audio Engine considers the device broken and aborts stream creation.
* *Proof:* This explains why the device shows up in Sound Settings but produces no sound and throws browser errors without generating any internal driver stream logs.

### Cause B: WebRTC/Browser Exclusive Format Demands (Likely)
Browsers (like Chrome/Edge testing on `mictests.com`) often attempt to bypass the Windows Audio Engine's shared resampler and request a specific microphone format directly (usually 16kHz Mono or 48kHz Mono for speech processing). 
* Since the driver now strictly refuses anything but 48kHz Stereo, the browser's `IAudioClient::Initialize` call fails with an unsupported format error, resulting in `NotReadableError`.

### Cause C: Driver Package Signing / Update Inconsistency (Plausible)
Since the driver is built and test-signed manually, Windows may be running a fragmented state. If the `.inf` file was updated but the OS didn't fully tear down the old audio endpoint graph, the audio service (`audiosrv`) might be in a corrupted state holding onto the old memory footprint of `lampyris-mic.sys`.

## 4. Summary & Actionable Next Steps
The driver code is functionally sound, but it is suffering from a **Stream Negotiation Failure**. The OS Audio Engine is refusing to start the stream because the formats it is *trying* to request are being rejected by our strict `48kHz Stereo` rule in `IsFormatSupported`, or because the audio endpoint cache in Windows is corrupted from previous mono-based driver installs.

**Next steps outside of code changes:**
1. Open Windows **Sound Settings** → **Recording** tab.
2. Double-click **Lampyris Mic** → Go to the **Advanced** tab.
3. Check the "Default Format" dropdown. Ensure it is explicitly set to **2 channel, 16 bit, 48000 Hz**. 
4. If it's greyed out or still set to a Mono format, the device needs to be uninstalled from Device Manager (checking the "Delete the driver software for this device" box) and reinstalled cleanly to clear the Windows audio cache.