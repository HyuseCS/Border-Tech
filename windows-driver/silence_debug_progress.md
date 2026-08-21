# Lampyris Mic — Silence Debug Progress (handoff)

> ## RESOLVED — 2026-08-21. Verified end to end on the Win10 22H2 VM.
>
> The mono pin fix was built, signed, installed and run. `NewStream ENTER Pin=1
> Capture=1` → `EXIT status=0x0`, SetState transitions fired, and live audio was
> audible with correct pitch.
>
> Three bugs, not one:
>
> 1. **Driver:** the MicIn capture pin advertised stereo only. Fixed — mono is
>    element 0 of `micinwavtable.h`.
> 2. **Client:** `pc-client/src/audio/windows.rs` packed each mono sample twice as
>    a fake L/R pair (left over from the stereo pin). Against a mono pin the driver
>    read each pair as two samples, so playback ran at half speed — audible as a
>    voice one octave too deep. Fixed — one sample per frame, byte counts `*2`.
> 3. **Latency:** `PushAudioData` in `lampyris_core.cpp` never bounded the queue.
>    The client pushes from the moment it connects but nothing drains the ring
>    until an app opens the mic, so it filled to its full 2 s and the reader
>    stayed that far behind all session — audible as hearing the mouse click that
>    started the recording. Fixed — drop the oldest bytes past a 100 ms watermark.
>
> Still open: the `External Microphone Headphone` (MicIn) endpoint still fails
> `GetMixFormat` because its cached registry `DeviceFormat` is stale at 2ch from
> the old build. Audio was confirmed through the temporary `MicArray1` diagnostic
> endpoint instead. Clear the cache with a full uninstall-with-driver-deletion +
> reinstall, or `reset_mic_endpoint.ps1`. Then remove `&MicArray1Miniports` from
> `minipairs.h` and strip the `LAMPYRIS-DEBUG` prints.

> ## ROOT CAUSE FOUND — 2026-08-21. Everything below this block is superseded.
>
> **The MicIn capture pin advertised stereo only. The Windows shared-mode capture
> pipe will not open a stereo-only capture endpoint.**
>
> Proven by a control experiment on a fresh Win10 22H2 VM: `&MicArray1Miniports`
> was temporarily added to `g_CaptureEndpoints`. On the *same driver binary*:
>
> | Endpoint | Formats | Result |
> |---|---|---|
> | MicIn | stereo only (2ch/48000/16) | `GetMixFormat -> AUDCLNT_E_UNSUPPORTED_FORMAT`, **zero `NewStream`** |
> | MicArray1 | mono default (1ch/48000/16) | `GetMixFormat -> S_OK`, `NewStream EXIT status=0x0`, SetState STOP→ACQUIRE→PAUSE |
>
> Commit `634691d` removed every mono format from MicIn. The silence dates from there.
>
> Mono is also required for a second, independent reason: `ReadAudioData` copies
> ring-buffer bytes straight into the DMA buffer with **no channel conversion**, and
> the wire protocol is mono 48kHz s16le. A stereo pin reads mono samples as L/R pairs.
>
> ### Refuted — do NOT re-investigate
>
> * **Stale cached `PKEY_AudioEngine_DeviceFormat`.** Measured on a clean install: the
>   cached value already read 2ch/48000/16, correctly serialized, and `GetMixFormat`
>   still failed. The byte-header anomaly was a red herring — the healthy control mic
>   has the same shape.
> * **The INF `PKEY_AudioEngine_DeviceFormat` write.** Built, signed, installed, tested.
>   The log was byte-for-byte the same shape: 26 `IsFormatSupported` calls, all
>   `STATUS_SUCCESS`, zero `NewStream`. No effect.
> * **The pin format table's *values*.** Driver, registry and control panel all agreed
>   on 2ch/48000/16. The problem was the *absence of a mono entry*, not a wrong value.
> * **`Microsoft-Windows-Audio/Operational`.** Event ID 65 only. No diagnostic value.
>
> ### Fix implemented 2026-08-21 — NOT YET COMPILED OR RUN
>
> * `micinwavtable.h` — mono 1ch/48000/16 inserted as element **0** (the default);
>   stereo kept as element 1.
> * `ComponentizedAudioSample.inx` — both `DeviceFormat` and `OEMFormat` blobs → mono.
> * `reset_mic_endpoint.ps1` — blob updated to match; endpoint matcher fixed to read
>   `PKEY_DeviceInterface_FriendlyName`, since `DeviceDesc` is the generic
>   "External Microphone Headphone" and never contains "Lampyris".
> * `minipairs.h` — `&MicArray1Miniports` left in as a positive control.
>   **Remove before release.**
>
> ### Other traps found
>
> * Update the driver on **Lampyris Virtual Microphone** under *Sound, video and game
>   controllers*. The "External Microphone Headphone (Lampyris Virtual Microphone)"
>   entry under *Audio inputs and outputs* is the endpoint — no INF matches it.
> * `TestApp.exe` is dead code. It expects `IOCTL_LAMPYRIS_AUTHENTICATE` and
>   `HKLM\SOFTWARE\Lampyris`; neither exists any more.
> * The four APO projects and `KeywordDetectorContosoAdapter` cannot build — their
>   headers were never vendored. Build `TabletAudioSample.vcxproj` alone.


**Next session starting fresh?** Read
`process/features/windows-driver/active/mic-deviceformat-fix_21-08-26/mic-deviceformat-fix_HANDOFF_21-08-26.md`
first, then follow `windows-driver/WINDOWS_VM_GUIDE.md` for VM steps.

**Symptom:** virtual mic outputs silence — green meter never moves, browsers
throw `NotReadableError`, Voice Recorder sees "no microphone". Producer
(PC client → IOCTL → ring buffer) works fine.

## Current state (latest capture)

- **The device now STARTS cleanly.** With a clean full-solution rebuild, the
  start-path instrumentation shows **every** step returning `0x0`:
  `StartDevice: AdapterCommon->Init / PcRegisterAdapterPowerManagement /
  InstallAllRenderFilters`, both `InstallSubdevice TopologyMicIn` and
  `WaveMicIn` (CreateAudioInterface → PcNewPort → MiniportCreate → port->Init →
  PcRegisterSubdevice all `0x0`), `ConnectTopologies -> 0x0`,
  `InstallAllCaptureFilters -> 0x0`.
- **The earlier `0xC00000BB` (STATUS_NOT_SUPPORTED) failed-start was a STALE
  BINARY**, not a real bug. A single-project build had been relinking a stale
  `EndpointsCommon.lib`; the clean solution rebuild fixed the start. Do NOT keep
  chasing the start failure.
- **But the mic still won't open.** Even during a real WASAPI open ("Listen to
  this device"), the log shows format enumeration only — the engine probes the
  1ch/2ch matrix, accepts `2ch/48000/16bit -> 0x0`, and then **`NewStream` is
  STILL never called** (no `DRI`, no `SetState`, no drain). The engine aborts
  the open *before* reaching our miniport.

So we are back to the ORIGINAL bug, now from a healthy-start baseline:
**a capture stream never opens.** The failed-start detour was a red herring.

## What's been ruled OUT (by evidence, don't re-try these)

- **Device start / registration.** All start steps return `0x0` (see above).
- **Wave format table.** Pre-pivot (mono-heavy) and current (stereo-only) configs
  fail identically; format enumeration works and accepts `2ch/48000/16bit`.
- **APOs.** MicIn endpoint has zero FX/APO references in the INF.
- **Topology descriptor.** `micintoptable.h` nodes/connections consistent.
- **WaveRT miniport `Init`.** Returns SUCCESS for a capture device.
- **Mic privacy settings.** All on.
- **ffmpeg / DirectShow (`Unable to BindToObject`, `(none)` tag).** RED HERRING —
  DirectShow's legacy KsProxy handles WaveRT capture endpoints poorly. The real
  clients (browser, Voice Recorder, WASAPI) use a different path. Stop using
  ffmpeg as the probe.
- **Event-driven (pull) mode mismatch.** Former leading suspect, now ruled out:
  the endpoint opts into event-driven capture via
  `PKEY_AudioEndpoint_Supports_EventDriven_Mode`, and the theory was that the
  polled `ExSetTimer`/`GetPosition` drain broke the notification contract. It is not
  the cause — the engine aborts at `GetMixFormat`/format-cache time, before any
  `Initialize(EVENTCALLBACK)` path is reached. See "Confirmed root cause" below.

## Confirmed root cause

**A stale cached `PKEY_AudioEngine_DeviceFormat` on the capture endpoint.**

`IAudioClient::GetMixFormat` on a **capture** endpoint reads the cached registry value
`PKEY_AudioEngine_DeviceFormat` (`{F19F064D-082C-4E27-BC73-6882A1BB8E4C},0`) under:

```
HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Capture\{endpoint-GUID}\Properties
```

It **never queries the driver**. So the pin's format table can be perfect and the mic
still won't open.

How it got stale:

- Commit `634691d` (21 Jun) collapsed the MicIn pin's wave-format table down to a single
  stereo `2ch / 48000 Hz / 16-bit` format. The endpoint's `Properties` key kept the older
  mono-era value.
- Commit `a86f997` added `PKEY_AudioEngine_OEMFormat`
  (`{E4870E26-3CC5-4CD2-BA46-CA0A9A70ED04},3`) to the INF with a byte-correct stereo blob.
  That was correct but ineffective here: **OEMFormat only seeds DeviceFormat the first
  time an endpoint's property store is created.** This endpoint's store already existed
  (created 30 Jun per `EventViewerLogs.md`), so the seed never applied.
- The INF never wrote `DeviceFormat` directly.

Net effect: the engine believes the mix format is stale/unsupported, aborts the capture
open **before** calling `NewStream`, and the meter never moves — exactly the observed
symptom.

**The fix (two halves, both required):**

1. `ComponentizedAudioSample.inx` now writes `PKEY_AudioEngine_DeviceFormat` directly in
   `[SYSVAD.I.TopologyMicIn.AddReg]`, with the same 48-byte stereo blob as OEMFormat and
   clobber flag `0x00000001`. This fixes **future clean installs**.
2. `windows-driver\reset_mic_endpoint.ps1` writes the same value into the live registry.
   This fixes **the currently-broken install** — the INF alone cannot, because the
   property store already exists.

Design decision: **keep the stereo 2ch/48000/16 contract.** Do not revert to mono —
`pc-client/src/audio/windows.rs` already upmixes mono to stereo, and the pin tables,
jack descriptor, and INF blob are all internally consistent at stereo.

## NEXT ACTION — verification procedure (steps a-d need NO driver rebuild)

Ordering is deliberate. Steps (a)-(d) test the theory and the registry-level fix without
rebuilding anything. Only after they pass does (e) rebuild the driver to make the fix
durable for future installs. Do not reorder.

**a. Run the reset script (elevated):** `.\reset_mic_endpoint.ps1`

Capture the printed **BEFORE** decode — this is the experiment.

- *Expected (confirms root cause):* `DeviceFormat: <not set>`, or it decodes to mono /
  non-48000 Hz / non-16-bit — i.e. stale relative to the current stereo pin table.
- *If BEFORE already shows `channels=2, rate=48000, bits=16, mask=0x3 (STEREO)`:* the root
  cause is **refuted** on this machine. Stop. Do not run b-e. Re-open the investigation.

**b. Re-run the probe:** `wasapi_probe.exe`

It now prints the endpoint id plus raw + decoded `DeviceFormat` and `OEMFormat` for every
capture endpoint, before it tries `GetMixFormat`.

- *Expected:* `GetMixFormat -> S_OK` returning `2 ch, 48000 Hz, 16 bit`, and shared-mode
  `Initialize` succeeding (`S_OK`) both with and without `EVENTCALLBACK`.
- *If `GetMixFormat` still fails:* the write in (a) did not take effect — check for a
  service-restart or permission problem and re-run (a) before continuing.

**c. Watch DebugView during a real open** (browser "Listen to this device", or Voice
Recorder).

- *Expected:* `NewStream`, `AllocateAudioBuffer` and `SetState` LAMPYRIS-DEBUG lines appear
  for the first time.
- *If they still do not appear:* the engine is still aborting before the miniport — capture
  the exact HRESULT chain from `wasapi_probe.exe` and treat it as a new investigation.

**d. End-to-end audio test:** start `pc-client` + the Android app streaming, then test in a
browser mic page or Voice Recorder.

- *Expected:* audible playback, VU meter moves. **This is the acceptance bar.**
- *If silent but (c) passed:* revisit the ring-buffer / `ReadAudioData` consumption path in
  `minwavertstream.cpp` — a new, narrower bug, not this one.

**e. Only after a-d all pass — make it durable for clean installs:**

- Clean **whole-solution** rebuild:
  `msbuild sysvad.sln /p:Configuration=Release /p:Platform=x64`
  (never a single project — see the stale `EndpointsCommon.lib` trap below).
- Re-sign: `sign_driver.ps1`.
- Uninstall the Lampyris mic device **with** "Delete the driver software for this device".
- Reinstall from the freshly built + signed `SignedPackage\ComponentizedAudioSample.inf`.
- Repeat (b) and (d) on the CLEAN install with **no manual registry step** — that proves the
  INF fix alone is now sufficient.


## Build/capture reminders (gotchas already hit)

- Build the **whole solution** (`msbuild sysvad.sln /p:Configuration=Release
  /p:Platform=x64`). Single-project builds relink a STALE `EndpointsCommon.lib`
  (no `<ProjectReference>` from TabletAudioSample) — this exact trap caused the
  phantom `0xC00000BB` failed-start. Always build the solution.
- Build **Release** (`sign_driver.ps1` hardcodes `x64\Release`).
- Install `SignedPackage\ComponentizedAudioSample.inf` (regenerated by the build
  + sign). Ignore the APO and Extension INFs — not needed for the mic.
- APO subprojects fail to build — not needed for `lampyris-mic.sys`.
- Start-path prints fire at **device start** (boot/install/enable); format-path
  prints fire at **mic open**. Capture with DebugView running at the right moment.

## Instrumentation locations (remove later: `grep -rn "LAMPYRIS-DEBUG"`)

- `adapter.cpp` — StartDevice step results.
- `common.cpp` — InstallSubdevice steps, ConnectTopologies.
- `EndpointsCommon/minwavert.cpp` — DataRangeIntersection, NewStream, IsFormatSupported.
- `EndpointsCommon/minwavertstream.cpp` — AllocateAudioBuffer, SetState.

(`windows-driver/tools/wasapi_probe.cpp` is a standalone diagnostic, not driver
code — delete or keep as you like.)
