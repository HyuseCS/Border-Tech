# Lampyris Mic — Windows Capture Post-Mortem

**Status: RESOLVED, verified on a Win10 22H2 VM on 2026-08-21.** One capture
endpoint, opens in shared / event-driven / exclusive mode, correct pitch, no
perceptible delay. Confirmed in Voice Recorder and in a live Discord call with
the voice-activity meter moving.

This file replaces the old live debugging notes. It exists so nobody re-chases
what was already eliminated.

## Four separate bugs

They were found in this order, and each one hid the next.

| # | Where | Symptom | Fix |
|---|---|---|---|
| 1 | `micinwavtable.h` | total silence, `NewStream` never called | the capture pin advertised **stereo only**; the shared-mode capture pipe will not open such an endpoint. Mono is element 0. |
| 2 | `pc-client/src/audio/windows.rs` | words correct, voice an octave too deep | the client wrote each mono sample twice as a fake L/R pair, left over from the stereo pin. Against a mono pin the driver read each pair as two samples, so playback ran at half speed. One sample per frame. |
| 3 | `lampyris_core.cpp` | fixed 1–2 s delay; you could hear the mouse click that started the recording | the ring buffer was unbounded. The client pushes from the moment it connects but nothing drains until an app opens the mic, so it filled to its full 2 s and the reader stayed that far behind. 100 ms watermark. |
| 4 | `minipairs.h` | `GetMixFormat -> AUDCLNT_E_UNSUPPORTED_FORMAT` | MicIn's own descriptor set could not be opened by the audio engine. It now uses MicArray's topology descriptor, wave descriptor, packet-size constraints and format/mode table, keeping only its own identity. |

Bug 4 was the hard one. The decisive evidence was a control experiment: adding a
second capture endpoint (`MicArray1`) to the same driver binary. It opened while
MicIn did not, which proved the fault was MicIn-specific configuration rather
than anything driver-wide. `IsFormatSupported(EXCLUSIVE, 1ch/48000/16)` returned
`S_OK` on MicIn throughout, so the pin was never the problem — only how the
endpoint was described to the audio engine.

## Ruled out — do not re-investigate

Every one of these was tested and killed:

- **Stale cached `PKEY_AudioEngine_DeviceFormat`** — measured correct on a clean
  install while the open still failed.
- **The INF `DeviceFormat` write** — built, installed, tested; no effect.
- **Forcing a 16-bit PCM blob as the mix format** — a capture mix format is
  32-bit float, but removing the override changed nothing either.
- **Missing WaveRT packet-size constraints** — added; no effect on its own.
- **A stale driver-store install** — the device was bound to the newest package
  the whole time (`pnputil /enum-devices /class Media /drivers` confirms this).
- **`MICIN_DEVICE_MAX_CHANNELS`** — it *was* a real regression (stock is 1, the
  stereo experiment set it to 2) and is fixed, but it was not the blocker.
- **Event-driven vs push notification contract, malformed `WAVEFORMATEXTENSIBLE`,
  wrong `dwChannelMask`, wrong pin index, `PROPOSEDATAFORMAT2` mode handling,
  the IOCTL/ring-buffer data path, the driver start path** — all verified sound.
- **An earlier `0xC00000BB` failed start** — a stale binary from a single-project
  build relinking a stale `EndpointsCommon.lib`. Not a real bug.
- **ffmpeg / DirectShow probes** — red herring; legacy `KsProxy` mishandles
  WaveRT capture. Real clients use a different path.

`DeviceFormat` and `OEMFormat` under MMDevices are **written by the audio
engine**, not by our INF — the serialized header shape differs from what AddReg
produces. They are output, not input. The endpoint that worked all along had a
nonsense value there (`1ch/8000/16`).

## Two Windows facts worth keeping

**Pin custom names must go in the global key.** Windows resolves
`KSPROPERTY_PIN_NAME` against
`HKLM\SYSTEM\CurrentControlSet\Control\MediaCategories\{GUID}\Name`. The stock
SYSVAD `HKR,%MEDIA_CATEGORIES%\...` form writes to the device's software key,
which nothing reads. Symptom: the endpoint falls back to the node-type default
name, e.g. "Microphone Array".

**Always purge the driver store before installing.** Five stale
`ComponentizedAudioSample` packages accumulated at one point and it was unclear
which the device was bound to. `rebuild_install.ps1` now does this automatically.

## Build and install

```
cd Z:\windows-driver
powershell -ExecutionPolicy Bypass -File .\rebuild_install.ps1
```

Elevated. Does build → sign → purge → install → reboot prompt, and judges the
build by whether `lampyris-mic.sys` was produced.

Build the **whole solution**, never a single project: `TabletAudioSample.vcxproj`
pulls in `EndpointsCommon.lib` as a raw link input with no `<ProjectReference>`,
so a single-project build silently relinks a stale library and discards your
changes. This trap produced the phantom `0xC00000BB` above.

## Verification runbook

1. `mmsys.cpl` → Recording — exactly one device, `Lampyris Virtual Microphone (Lampyris Mic)`.
2. `windows-driver\tools\wasapi_probe.exe` — `GetMixFormat -> S_OK, 1 ch, 48000 Hz`, and all three deep-probe opens succeed.
3. Phone streaming + `lampyris.exe` + Voice Recorder — audible, correct pitch, no delay.
4. Discord or Teams mic test — exercises COMMUNICATIONS mode, which is a separate
   format-negotiation path. Every mode is pinned to 48 kHz mono because the
   driver does no resampling; MicArray's stock table mapped SPEECH to 16 kHz and
   COMMUNICATIONS to 24 kHz, which would play back at the wrong pitch.

## Tracing

All `[LAMPYRIS]` output is gated behind `LAMPYRIS_TRACE` in
`lampyris-sysvad/lampyris_debug.h`, off by default. Set it to 1 and rebuild to
get the DebugView trace back. Run DebugView as Administrator with kernel capture
on; start-path prints fire at device start, format-path prints at mic open.

Captures from the investigation are in `debug_logs/` — `WINDOWSVM7.log` is the
first one where MicIn opens.
