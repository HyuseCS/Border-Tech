# windows-driver

<!-- Part of Project-M -->

## Scope

The Lampyris Mic kernel-mode virtual microphone for Windows (`windows-driver/`). Covers the WDM driver that receives PCM over IOCTL and buffers it, the SYSVAD-derived audio miniport that exposes a capture endpoint to the Windows audio engine, and the test-signing / certificate-trust tooling.

Does NOT cover the user-mode sender (`pc-client/src/audio/windows.rs`) — but the two share `ioctl.h`, so any change to the IOCTL contract must land on both sides in the same commit.

On Linux this whole component is unnecessary: PipeWire provides the virtual source in user space.

## Key Source Files

- `windows-driver/driver.cpp` (242 loc) — `DriverEntry`, `LampyrisDeviceControl` IOCTL dispatch, `PushAudioData`, and a 2-second PCM ring buffer (`48000 * 2 * 2` bytes) guarded by a `KSPIN_LOCK`
- `windows-driver/ioctl.h` — **the shared contract.** Device type `0x8001`, function `0x802`, `METHOD_BUFFERED | FILE_WRITE_ACCESS`, payload `{ ULONG Length; BYTE Data[9600]; }`
- `windows-driver/lampyris-sysvad/` — SYSVAD-derived audio miniport (`EndpointsCommon/`, `APO/`, `Package/`, `TabletAudioSample/`, `KeywordDetectorAdapter/`), own `sysvad.sln`
- `windows-driver/lampyris-mic.sln` / `.vcxproj` — MSBuild projects for the WDM driver
- `windows-driver/TestApp.cpp` — user-mode harness for poking the IOCTL directly
- `windows-driver/*.ps1` — `sign_driver.ps1`, `trust_certs.ps1`, `export_cert.ps1`, `find_cert.ps1`. Manual and machine-specific.

## Debug Notes (read these first)

- `windows-driver/silence_debug_progress.md` — **live handoff for the current bug**, including the ruled-out list
- `windows-driver/build_errors.md` — build failure history
- `windows-driver/debug_capture_instructions.md` — how to capture driver logs

## Related Context

- `process/context/all-context.md` — the IOCTL and wire contract
- `process/context/tests/all-tests.md` — no automated tests exist for this component
- `process/features/pc-client-lampyris/_GUIDE.md` — the user-mode side of the bridge

## Gotchas

- **Always do a clean full-solution rebuild.** A single-project build once relinked a stale `EndpointsCommon.lib` and produced a fake `0xC00000BB` (`STATUS_NOT_SUPPORTED`) start failure that cost a whole debugging detour. That failure is NOT a real bug — do not re-chase it.
- Kernel mode: a mistake bugchecks the machine. Prefer a VM.
- `ioctl.h` is shared with the Rust side. Changing the struct, the device type, or the function code silently breaks `pc-client/src/audio/windows.rs`.
- There are no automated tests. Verification is: build clean, install, open a capture app, watch the meter.

## Current Status

Status: **root cause identified; fix implemented but NOT yet user-verified** (pending a run on the Windows VM).

**Symptom.** The virtual mic outputs silence. The device starts cleanly (every start-path step returns `0x0`) and the audio engine accepts the `2ch/48000/16bit` format, but `NewStream` is never called — the engine aborts the open before it reaches the miniport. The green meter never moves, browsers throw `NotReadableError`, Voice Recorder reports "no microphone". The producer side is confirmed healthy.

**Root cause (confirmed).** A stale cached `PKEY_AudioEngine_DeviceFormat` (`{F19F064D-082C-4E27-BC73-6882A1BB8E4C},0`) on the capture endpoint under `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Capture\{guid}\Properties`. `IAudioClient::GetMixFormat` on a capture endpoint reads that registry value and never queries the driver. `PKEY_AudioEngine_OEMFormat` only *seeds* it when the property store is first created, so the earlier OEMFormat fix could not repair this already-existing endpoint. The engine therefore sees a stale mix format and aborts the open before `NewStream`.

**Fix implemented (two halves, both required).**

1. `ComponentizedAudioSample.inx` now writes `PKEY_AudioEngine_DeviceFormat` directly in `[SYSVAD.I.TopologyMicIn.AddReg]` with the same 48-byte stereo blob as OEMFormat and clobber flag `0x00000001` — fixes **future clean installs**.
2. `windows-driver/reset_mic_endpoint.ps1` (new) writes the same value into the live registry, with elevation check, positive endpoint matching, `.reg` backup, BEFORE/AFTER decode, and an opt-in `-Purge` mode — fixes **the currently-broken install**.

`windows-driver/tools/wasapi_probe.cpp` now prints the decoded `DeviceFormat`/`OEMFormat` for every capture endpoint, so the stale value is visible directly in the probe output.

**Still to do (user-executed on the Windows VM).** Run the verification runbook in `windows-driver/silence_debug_progress.md` ("NEXT ACTION"): (a) reset script + record the BEFORE decode, (b) probe, (c) DebugView for `NewStream`, (d) end-to-end audible audio, (e) whole-solution rebuild + clean reinstall to prove the INF fix alone suffices. Nothing here is verified until (d) passes.

## Folder Contents

```
process/features/windows-driver/
  active/       -- in-progress plans for this feature (each task lives inside a {slug}_{date}/ task folder)
  completed/    -- archived completed plans
  backlog/      -- deferred/future plans
```

All artifacts (plans, specs, reports, references) colocate inside each `{slug}_{date}/` task folder. Do NOT create `reports/` or `references/` sibling dirs.
