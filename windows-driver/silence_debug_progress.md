# Lampyris Mic — Silence Debug Progress (handoff)

**Symptom:** virtual mic outputs silence — green level meter never moves, browsers
throw `NotReadableError`. Producer (PC client → IOCTL → ring buffer) works fine.

## Where we are

The audio **drain pipeline is wired correctly** (`IOCTL push → g_AudioRingBuffer
→ TimerNotifyRT → UpdatePosition → WriteBytes → ReadAudioData → m_pDmaBuffer`),
but it **never executes** because the OS never opens the capture stream.

We added temporary DbgPrint instrumentation (all tagged `// LAMPYRIS-DEBUG`) to
the open/negotiation path and captured a fresh DebugView log
(`debug_logs/WINDOWSVM.log`, 769 lines). Full-log stage tally:

| Stage | Count | Meaning |
|------|------:|---------|
| `IsFormatSupported` | 752 | format probing only |
| `IOCTL_PUSH_AUDIO` | 16 | producer pushing audio (works) |
| `NewStream` | **0** | stream never instantiated |
| `DataRangeIntersection` | 0 | engine uses PROPOSEDATAFORMAT path instead |
| `AllocBuffer` | 0 | — |
| `SetState` | 0 | never reaches RUN |
| drain (ReadAudioData/Timer/GetPosition) | 0 | never runs |

## Confirmed / refuted

- **REFUTED — mono-rejection theory.** `IsFormatSupported` *accepts*
  `2ch/48000Hz/16bit -> 0x0` (SUCCESS) and rejects all else (`0xC0000272` =
  `STATUS_NO_MATCH`). Format negotiation is healthy; the driver is fine with
  stereo 48k. `DataRangeIntersection` is never even called.
- **CONFIRMED mechanism.** `IsFormatSupported` is called *from* `NewStream`
  (minwavert.cpp:701), yet `NewStream` count is 0 → all 752 hits are pure
  enumeration, **no stream open ever attempted**. No `NewStream` → no
  `SetState(RUN)` → timer never starts → ring fills to 192000 and never drains →
  silence.
- **Failure location:** *above* our miniport. The Windows audio engine
  enumerates formats, picks one, then aborts `IAudioClient::Initialize`
  **before** calling our driver's `NewStream`. That pre-NewStream abort is what
  surfaces as the browser `NotReadableError`.

## Leading causes (next to investigate)

1. **Stale cached default format (top suspect).** `HKLM\...\MMDevices\Audio\
   Capture` may still hold a **mono** default from earlier mono-era installs.
   Engine tries to Initialize at mono → driver rejects mono → bails before
   NewStream, even though stereo would work. Only a true clean uninstall clears
   it. OPEN QUESTION: did the last reinstall include Device Manager → "Delete
   the driver software for this device" (or `pnputil /delete-driver`)? If it was
   an install-over-the-top, the cached mono default survives.
2. **Single rigid format.** Offering only 48k stereo gives the engine no mono
   option for the many capture clients that want mono. MS comment in
   `DataRangeIntersection` says you must add a separate mono data range to
   support mono.

## Next steps (in priority order)

1. **Get the real error code first (cheapest, highest value).** On the VM:
   Event Viewer → Applications and Services Logs → Microsoft → Windows → Audio
   (Operational) + the System log, at the timestamp of the open attempt. The
   `AUDCLNT_*` / NTSTATUS there names the exact failure — beats another build
   cycle of guessing.
2. **Confirm a truly clean reinstall** (uninstall + delete driver software →
   reboot → reinstall) to flush any cached mono default, then re-capture.
3. **If it still won't open:** re-add a mono (1ch/48k/16-bit) format + matching
   `KSDATARANGE_AUDIO` to `TabletAudioSample/micinwavtable.h` so the engine can
   open the endpoint directly in mono. The PC client has the real mono signal
   before it duplicates to stereo, so a mono endpoint is closer to the source
   anyway. (The stereo-only pivot was the regression that started this.)

## Build/capture reminders (gotchas already hit)

- Build the **whole solution** (`msbuild sysvad.sln`) or build
  `EndpointsCommon.vcxproj` **then** `TabletAudioSample.vcxproj` — the latter has
  no `<ProjectReference>` to EndpointsCommon, so building it alone relinks a
  **stale** `EndpointsCommon.lib` and silently drops changes to
  `minwavert.cpp` / `minwavertstream.cpp`. (This wasted one capture cycle.)
- APO subprojects (KWSApo/AecApo/DelayAPO/SwapAPO/KeywordDetector) fail to build
  (missing sample headers/IDL) — not needed for `lampyris-mic.sys`; build
  EndpointsCommon + TabletAudioSample directly. See `build_errors.md`.
- Build **Release** (sign_driver.ps1 hardcodes `x64\Release`).
- During capture, actually open a capture client (Sound settings "Listen to this
  device" / mictests / Voice Recorder) — IOCTL pushes happen regardless of any
  consumer.

## Instrumentation locations (to remove later: `grep -rn "LAMPYRIS-DEBUG"`)

- `EndpointsCommon/minwavert.cpp` — DataRangeIntersection, NewStream ENTER/EXIT,
  IsFormatSupported result.
- `EndpointsCommon/minwavertstream.cpp` — AllocateAudioBuffer, SetState.

See `debug_capture_instructions.md` for full build/sign/capture steps.
