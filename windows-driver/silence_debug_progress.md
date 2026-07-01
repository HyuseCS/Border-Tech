# Lampyris Mic — Silence Debug Progress (handoff)

**Symptom:** virtual mic outputs silence — green meter never moves, browsers
throw `NotReadableError`, Voice Recorder sees "no microphone". Producer
(PC client → IOCTL → ring buffer) works fine.

## ROOT CAUSE FOUND — the device fails to start

Device Manager → Lampyris Virtual Microphone → **Events** tab:

```
Device ROOT\MEDIA\0000 had a problem starting.
Service: sysvad_componentizedaudiosample
Problem: 0x15            (CM_PROB_FAILED_START — "device cannot start")
Problem Status: 0xC00000BB  (STATUS_NOT_SUPPORTED)
```

The PortCls **audio adapter never starts**, so there is no functional KS capture
filter. That's why:
- ffmpeg `-list_devices` tags the mic **`(none)`** (not `(audio)`), and opening
  it fails with **`Unable to BindToObject`**.
- WASAPI `NewStream` is **never** called (no pin to open).
- Sound settings still *shows* the endpoint (stale MMDevice registration) with a
  greyed, unchangeable default format.

The IOCTL producer still works because its control device (`\Device\LampyrisMic2`)
is created in `DriverEntry`, independent of the failing PortCls adapter.

## What's been ruled OUT (by evidence, don't re-try these)

- **Wave format table.** The pre-pivot config (jack=MONO + many mono ranges) and
  the current stereo-only config **both fail identically**. Format enumeration
  works and accepts `2ch/48000/16bit -> 0x0`. Not the cause. (Git: commit
  `634691d` did the mono→stereo pivot; it did NOT fix the start failure.)
- **APOs.** The MicIn endpoint has zero FX/APO references in the INF, so the APO
  subproject build failures are irrelevant to it.
- **WaveRT miniport `Init`.** Returns SUCCESS for a capture device (skips the
  whole render/offload block; MicIn has no audio modules).
- **Topology descriptor.** `micintoptable.h` nodes (VOLUME/MUTE/PEAKMETER) and
  connections are consistent; `C_ASSERT`s hold.
- **Mic privacy settings.** All on (checked in Windows Settings).

## Where the failure is (narrowed)

Only ONE endpoint is registered: `MicInMiniports` (minipairs.h;
`g_cRenderEndpoints == 0`). So the failing call is inside:

`StartDevice` (adapter.cpp) → `InstallAllCaptureFilters` →
`InstallEndpointFilters(MicIn)` (common.cpp) →
`InstallSubdevice(Topo)` / `InstallSubdevice(Wave)` →
{`CreateAudioInterfaceWithProperties` → `PcNewPort` → `MiniportCreate` →
`port->Init` → `PcRegisterSubdevice`} → `ConnectTopologies`.

`port->Init` and `PcRegisterSubdevice` are PortCls black boxes — can't pin the
exact line by reading. So the start path is now **instrumented** (see below).

## Current step: start-path instrumentation (awaiting a capture)

Added `DbgPrint`s tagged `// LAMPYRIS-DEBUG` at every step of the start path:
- `adapter.cpp` `StartDevice` — after Init / power / render / capture installs.
- `common.cpp` `InstallSubdevice` — after CreateAudioInterface / PcNewPort /
  MiniportCreate / port->Init / PcRegisterSubdevice (each tagged with the
  subdevice Name: `TopologyMicIn` or `WaveMicIn`).
- `common.cpp` `InstallEndpointFilters` — after `ConnectTopologies`.

**Next action (on Windows):** build the whole solution → sign → install →
run DebugView (Capture Kernel) → **Device Manager: Disable then Enable the
Lampyris mic** to re-run `StartDevice` → save log. The first print showing
`-> 0xC00000BB` names the exact failing call. Full steps + decision tree in
`debug_capture_instructions.md` §3–4.

## Build/capture reminders (gotchas already hit)

- Build the **whole solution** (`msbuild sysvad.sln /p:Configuration=Release
  /p:Platform=x64`). `adapter.cpp`/`common.cpp` compile into the `.sys`, but the
  earlier `minwavert.cpp`/`minwavertstream.cpp` prints live in
  `EndpointsCommon.lib` (no `<ProjectReference>` from TabletAudioSample), so a
  single-project build silently drops them. Build the solution.
- Build **Release** (`sign_driver.ps1` hardcodes `x64\Release`).
- APO subprojects (KWSApo/AecApo/DelayAPO/SwapAPO/KeywordDetector) fail to build
  — not needed for `lampyris-mic.sys`. See `build_errors.md`.
- Start-path prints fire at **device start**, not mic-open — capture while
  disabling/enabling the device.

## Instrumentation locations (remove later: `grep -rn "LAMPYRIS-DEBUG"`)

- `adapter.cpp` — StartDevice step results.
- `common.cpp` — InstallSubdevice steps, ConnectTopologies.
- `EndpointsCommon/minwavert.cpp` — DataRangeIntersection, NewStream, IsFormatSupported (format-path, earlier).
- `EndpointsCommon/minwavertstream.cpp` — AllocateAudioBuffer, SetState (format-path, earlier).
