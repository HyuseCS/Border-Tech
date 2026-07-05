# Lampyris Virtual Mic — Root-Cause Analysis: Why the Windows Driver Won't Open for Capture

**Date:** 2026-07-05
**Scope:** `windows-driver/` (SYSVAD-derived `lampyris-mic.sys`) + `pc-client/` (Rust).
**Symptom:** Device starts cleanly, format enumeration succeeds, producer path (client → IOCTL → ring buffer) works — but the Windows audio engine **never calls `IMiniportWaveRT::NewStream`**. No capture stream ever opens: browsers throw `NotReadableError`, Voice Recorder sees "no microphone", "Listen to this device" is silent.

Evidence sources: full code inspection of both trees, `debug_logs/WINDOWSVM.log` (DebugView capture), `debug_logs/EventViewerLogs.md`, and a documentation/community research sweep (Microsoft Learn, sysvad source, OSR/wdmaudiodev, VAC vendor docs, Chromium source). Citations inline.

---

## 1. Verdict

**The leading suspect in `silence_debug_progress.md` — the event-driven (pull-mode) mismatch — is refuted, on two independent grounds:**

1. **The code implements the notification contract.** `CMiniportWaveRTStream` inherits and answers QI for `IMiniportWaveRTStreamNotification` (`lampyris-sysvad/EndpointsCommon/minwavertstream.h:45-54`, `minwavertstream.cpp:485-488`), implements `AllocateBufferWithNotification` (`minwavertstream.cpp:528` — MDL via `AllocatePagesForMdl`, sets `m_ulNotificationsPerBuffer`/`m_ulNotificationIntervalMs`) and `RegisterNotificationEvent`/`UnregisterNotificationEvent` (`minwavertstream.cpp:641/685`). `TimerNotifyRT` signals the registered `KEVENT`s (`minwavertstream.cpp:1868`). The "polled `ExSetTimer` drain" and the notification contract coexist — that is stock sysvad's own pattern.
2. **The contract is only probed *after* `NewStream`.** Per Microsoft's WaveRT documentation, the open sequence is: the engine opens a pin on the KS filter (→ `NewStream` creates the stream), *then* portcls QIs the returned stream object for `IMiniportWaveRTStreamNotification`, *then* the buffer/notification properties are exchanged. Every RTAUDIO notification property (`KSPROPERTY_RTAUDIO_BUFFER_WITH_NOTIFICATION`, `REGISTER_NOTIFICATION_EVENT`, `QUERY_NOTIFICATION_SUPPORT`) targets a **pin instance**, which cannot exist before `NewStream` has succeeded. A driver lacking notification support fails *after* pin creation (STATUS_NOT_SUPPORTED on the pin property), never before it.
   - https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/understanding-the-wavert-port-driver
   - https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/portcls/nn-portcls-iminiportwavertstreamnotification
   - https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/ksproperty-rtaudio-buffer-with-notification

**Conclusion:** the open is aborting **inside the Windows audio engine (AudioSrv / audiodg.exe), above portcls, before pin creation**. Nothing in the driver's miniport, stream, format-table, or topology code has a gate that can produce "formats accepted but `NewStream` never called" — each candidate gate was inspected and is correct (§3). The remaining candidates are engine-side state and KS-layer validation (§4).

---

## 2. System architecture (as verified)

```
Phone (Android)                    PC client (Rust)                      Virtual mic
────────────────                   ──────────────────                    ─────────────────────────────
16-bit mono 48kHz PCM   ──TLS──►   decode to f32 mono         Linux:    PipeWire "Audio/Source" node,
'MC' framing, seq nums   TCP       (app_state.rs:458-505,     (works)   mono F32 48kHz, pull model
SRP auth, TOFU pinning             protocol.rs)                         (audio/linux.rs:64-167)
                                        │
                                        ▼                     Windows:  \\.\LampyrisMic2 (CreateFileW)
                                   WindowsSink converts       (broken)  IOCTL_LAMPYRIS_PUSH_AUDIO
                                   mono f32 → stereo i16                → 192,000-byte ring buffer
                                   (dup L/R, windows.rs:                → WaveRT capture stream drains
                                   131-137), ≤9600 B/push                 via ReadAudioData
```

**Windows driver composition.** `lampyris-mic.sys` is sysvad's TabletAudioSample stripped to a **single MicIn capture endpoint** (`g_cRenderEndpoints = 0`, `g_CaptureEndpoints = { &MicInMiniports }` — `TabletAudioSample/minipairs.h:510-527`), plus a custom graft `lampyris_core.cpp`:
- Control device `\Device\LampyrisMic2` + `\DosDevices\LampyrisMic2` symlink, SDDL `D:P(A;;GA;;;SY)(A;;GA;;;IU)` (`lampyris_core.cpp:156-185`).
- 192,000-byte spinlock-guarded ring buffer; `PushAudioData` (IOCTL side, `:29-55`) overwrites oldest on overflow; `ReadAudioData` (`:57-89`) zero-fills underruns.
- **IRP dispatch hook**: after `PcInitializeAdapterDriver`, `LampyrisInit` saves and replaces the driver object's `MJ_CREATE/MJ_CLOSE/MJ_DEVICE_CONTROL` handlers (`:187-194`); non-control-device IRPs are forwarded to the saved portcls handlers (`:91-115`).
- The WaveRT capture stream is spliced into this ring: `CMiniportWaveRTStream::WriteBytes` calls `ReadAudioData` instead of sysvad's tone generator (`EndpointsCommon/minwavertstream.cpp:1519-1546`, invoked from `UpdatePosition` at `:1450-1454`).

**Client ↔ driver format compatibility: confirmed exact match.**
- Client pushes interleaved **2ch / 48,000 Hz / 16-bit LE** (mono f32 scaled to i16, written twice — `pc-client/src/audio/windows.rs:131-137`; rate fixed at 48000 in `app_state.rs:459`; `length = samples_read * 4` at `:147`).
- The endpoint's **sole** device format is 2ch / 48,000 / 16-bit, `KSAUDIO_SPEAKER_STEREO` (`TabletAudioSample/micinwavtable.h:34-61`).
- IOCTL codes/payload are hand-duplicated in `windows.rs:14-36` and currently match `ioctl.h:12-27` exactly (device 0x8001, function 0x802, METHOD_BUFFERED, FILE_WRITE_ACCESS, 9600-byte max payload).

**What the latest DebugView capture shows** (`debug_logs/WINDOWSVM.log`):
- Engine probes the format matrix on Pin 1 and accepts exactly `2ch/48000/16bit → 0x0`, rejecting everything else with `0xC0000272` (STATUS_NO_MATCH) — consistent with the single-entry format table.
- `IOCTL_LAMPYRIS_PUSH_AUDIO` prints (throttled to every 100th push) arrive ~3.4 s apart — i.e. ~34 ms per push of 50 ms of audio: the producer runs at (slightly faster than) real time. `Ring buffer available: 192000` = the ring is **full** and overwriting, because nothing ever drains it.
- **Zero** `NewStream`, `SetState`, `AllocBuffer`, or drain lines. The engine never reaches the miniport's open path.

---

## 3. What is proven working (do not re-litigate)

| Item | Evidence |
|---|---|
| Device start path | All steps `0x0`: adapter Init, power registration, `InstallSubdevice TopologyMicIn`/`WaveMicIn`, `ConnectTopologies`, `InstallAllCaptureFilters` (instrumented, per `silence_debug_progress.md`) |
| Format enumeration reaches the driver | `IsFormatSupported`/`DataRangeIntersection` prints fire and accept 2ch/48k/16 (`WINDOWSVM.log`) |
| Producer path | IOCTL pushes at real-time rate, ring fills (`WINDOWSVM.log`; `lampyris_core.cpp:126-147`) |
| Notification (event-driven) contract | Fully implemented (`minwavertstream.cpp:485-488, 528, 641`) — see §1 |
| Signal-processing modes | MicIn supports RAW, **DEFAULT**, SPEECH, COMMUNICATIONS, FAR_FIELD_SPEECH (`micinwavtable.h:66-89`); DEFAULT or RAW is the engine's minimum requirement ([audio-signal-processing-modes](https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/audio-signal-processing-modes)) |
| Mode attribute on the stream data range | `KSDATARANGE_ATTRIBUTES` flag set, attribute list attached (`micinwavtable.h:141,157-161`), and `PinDataRangeSignalProcessingModeAttribute` has **flags = 0, not `KSATTRIBUTE_REQUIRED`** (`EndpointsCommon/simple.h:161-180`) — exactly the WHQL cert requirement ([deviceaudio-requirements](https://learn.microsoft.com/en-us/previous-versions/windows/hardware/cert-program/deviceaudio-requirements)) |
| No APO/FX for MicIn in the current INX | `[SYSVAD.I.WaveMicIn.AddReg]` sets only FriendlyName; `[SYSVAD.I.TopologyMicIn.AddReg]` sets Association + EventDriven only (`ComponentizedAudioSample.inx:169-180`) |
| NewStream itself has no failing gate | Only `ValidateStreamCreate` and `IsFormatSupported` (`minwavert.cpp:705,712`) — both pass for the enumerated format |
| IRP hook forwards correctly for the paths we can observe | Filter opens (MJ_CREATE) and property IOCTLs (MJ_DEVICE_CONTROL) demonstrably reach the miniport — the same saved handlers would serve a pin create |
| ffmpeg/DirectShow failures | Red herring — legacy KsProxy mishandles WaveRT capture endpoints; real clients use WASAPI |
| The old `0xC00000BB` failed-start | Stale-binary artifact of single-project builds; fixed by full-solution rebuild |

---

## 4. Ranked root-cause hypotheses

Ranked with the constraint that the driver-side open path is verified correct, so the abort must occur in the engine or the KS layer above the miniport.

### #1 — Stale persisted endpoint state from months of broken installs (top suspect)

Windows keeps a **persistent per-endpoint PropertyStore** ("The same PropertyStore is available regardless of how often you restart your machine"), seeded at endpoint creation from the INF and the device-interface registry. A plain driver update **keeps the same endpoint GUID and its old properties**; only a full uninstall deletes them.
- https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/audio-endpoint-builder-algorithm
- https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/default-audio-endpoint-selection

Two persisted artifacts can abort an open before any pin is attempted:
- **Stale FX/APO references.** If any earlier install registered APO CLSIDs on the endpoint (the repo's git history shows `ComponentizedApoSample.inf` and APO subprojects were previously part of the built/signed package), audiodg tries to instantiate them at stream init ("software effects are inserted in the software device pipe on stream initialization") and the open fails when the DLLs are missing/unregistered. Windows only auto-disables broken sAPOs after **10 consecutive failures**. ([audio-processing-object-architecture](https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/audio-processing-object-architecture))
- **Stale/corrupt `PKEY_AudioEngine_DeviceFormat` blob.** Shared-mode Initialize negotiates against this *cached* blob, not a live driver query; a bad persisted blob demonstrably blocks opens and survives reboot, fixed only by uninstall/reinstall. ([pkey-audioengine-deviceformat](https://learn.microsoft.com/en-us/windows/win32/coreaudio/pkey-audioengine-deviceformat); [MSDN forum case](https://learn.microsoft.com/en-us/archive/msdn-technet-forums/e2e9858f-9bf5-4400-8a4e-570fa3285aad))

Important caveat from a Microsoft engineer (wdmaudiodev): deleting the `MMDevices` endpoint keys is not enough — the rebuilt endpoint **re-reads `HKR\EP\…` values from the still-installed driver package**, so every old lampyris/sysvad package must also be removed with `pnputil /delete-driver`. ([thread](https://www.freelists.org/post/wdmaudiodev/EXTERNAL-Re-Audio-driver-issues-No-speakers-or-headphones-are-plugged-in,2)) Stale endpoint devnodes also persist under `HKLM\SYSTEM\CurrentControlSet\Enum\SWD\MMDEVAPI`.

This hypothesis fits the project history precisely: the test machine has absorbed many installs of broken/differently-configured builds (APO variants, format-table pivots, failed-start builds), while the current driver code is verified clean.

### #2 — Engine-side abort identified only by the WASAPI probe HRESULT

`tools/wasapi_probe.cpp` (already written, never yet run) opens the endpoint exactly like Chromium does (shared mode, with and without `AUDCLNT_STREAMFLAGS_EVENTCALLBACK`) and prints the HRESULT at each step. Chromium collapses any Initialize failure into `NotReadableError` ([audio_low_latency_input_win.cc](https://chromium.googlesource.com/chromium/src/media/+/refs/heads/main/audio/win/audio_low_latency_input_win.cc)), so the probe's raw HRESULT is the single most informative datum obtainable without a rebuild:

| Probe result | Meaning | Points at |
|---|---|---|
| `AUDCLNT_E_ENDPOINT_CREATE_FAILED` (0x8889000F) | endpoint pump/APO graph/pin build failed in audiosrv+audiodg | #1 (FX residue), #3, #4 |
| `AUDCLNT_E_UNSUPPORTED_FORMAT` | mix-format negotiation vs cached DeviceFormat blob | #1 (format blob) |
| `AUDCLNT_E_DEVICE_IN_USE` | policy arbitration; no pin attempted | exclusive-mode claim / another client |
| `AUDCLNT_E_DEVICE_INVALIDATED` / `E_NOTFOUND` at activation | endpoint not ACTIVE | endpoint state (#1/AudioEndpointBuilder) |
| Open **succeeds** | driver + engine fine | pivot entirely to client/app permission side |

(HRESULT semantics: [IAudioClient::Initialize](https://learn.microsoft.com/en-us/windows/win32/api/audioclient/nf-audioclient-iaudioclient-initialize).)

### #3 — KsCreatePin rejected in ks.sys (mode-attribute path) — demoted by code inspection

The closest documented community match for "data intersection OK, yet the miniport is never called": Windows appends a `KSATTRIBUTEID_AUDIOSIGNALPROCESSING_MODE` attribute to the connect format for mode-aware drivers, and if a pin's data ranges lack the attribute list (or mark it `KSATTRIBUTE_REQUIRED`), `KsCreatePin` fails with STATUS_NO_MATCH **above the miniport**. ([wdmaudiodev field report](https://www.freelists.org/post/wdmaudiodev/KsCreatePin-Failure,1); cert rule in [deviceaudio-requirements](https://learn.microsoft.com/en-us/previous-versions/windows/hardware/cert-program/deviceaudio-requirements))

Inspection shows the MicIn tables are stock-correct on every point of that rule (§3), so this drops to third — revisit only if the probe fails in a way #1's cleanup doesn't cure.

### #4 — Pin CINSTANCES current-count nonzero

VAC's vendor documentation records a Windows engine behavior where the "System Audio Engine does not create a pin instance if CurrentCount is nonzero" on a capture pin — the engine silently never attempts the open. ([VAC winbugs](https://vac.muzychenko.net/en/manual/winbugs.htm)) Unlikely here (no stream was ever created, so the count should be 0), but it is cheap to check: query `KSPROPERTY_PIN_CINSTANCES` for Pin 1 (KsStudio, or one more DbgPrint in `PropertyHandler_Pin`).

### #5 — The IRP dispatch hook (`lampyris_core.cpp:187-194`)

Manually swapping the driver object's major-function pointers after `PcInitializeAdapterDriver` is a documented anti-pattern (OSR guidance is to forward to `PcDispatchIrp`; [OSR thread](https://community.osr.com/t/sysdriver-how-to-capture-input-data-and-play-user-audio-application-data/56546)). However, the observable evidence exonerates it for *this* symptom: filter creates and property IOCTLs demonstrably traverse the hook and reach the miniport, and a pin create is an `IRP_MJ_CREATE` on the same device object routed through the same saved handler (`lampyris_core.cpp:91-95`). Keep it on the list only if ETW shows the pin-create IRP arriving at the filter without portcls calling `NewStream`. Independent of this bug, replacing the hook with `PcDispatchIrp` forwarding is a worthwhile hardening.

### #6 — Event-driven PKEY mismatch — **refuted** (§1)

Keep `PKEY_AudioEndpoint_Supports_EventDriven_Mode = 1` (`ComponentizedAudioSample.inx:180`). It matches stock sysvad (which sets it on every endpoint), the WHQL pairing rule, and the implemented DDIs. Removing it would be a regression, not a fix.

---

## 5. Secondary defects (real bugs, but not the NewStream blocker — queue for later)

1. **Ring buffer is ~1 s of stereo, not "2 s of mono", and serves stale audio.** `RING_BUFFER_SIZE = 192000` with a mono comment (`lampyris_core.cpp:17-18`), but the client feeds stereo at 192,000 B/s. Worse, the full-ring overwrite semantics (`:48-50`) mean that when a capture stream finally opens, the first second delivered is the *oldest* buffered audio → up to ~1 s latency. Fix: flush (or fast-forward the read offset) when a capture stream opens/runs.
2. **Dead duplicate driver project.** Root `lampyris-mic.vcxproj` builds only `driver.cpp` — a standalone WDM driver with its own `DriverEntry` and device `\Device\LampyrisMic` (no "2"), duplicating the ring buffer. It is not the shipped driver (that is TabletAudioSample + `lampyris_core.cpp`) and both target the same output name — a stale-binary footgun of exactly the kind that produced the phantom `0xC00000BB`. Delete or clearly quarantine it.
3. **`TestApp.cpp` is stale.** It reads `HKLM\SOFTWARE\Lampyris\SessionToken` (nothing creates it) and sends `IOCTL_LAMPYRIS_AUTHENTICATE` (0x801), which `lampyris_core.cpp` has no case for → `STATUS_INVALID_DEVICE_REQUEST`. Only the PUSH_AUDIO leg reflects reality.
4. **IOCTL definitions hand-duplicated** in `pc-client/src/audio/windows.rs:14-36` vs `windows-driver/ioctl.h` — in sync today, with no compile-time guarantee.
5. **Client drops IOCTL failures silently** (log-and-continue, no reopen/backoff — `windows.rs:164-166`); acceptable for now, worth a reconnect path later.

---

## 6. Recommended action plan (ordered, cheapest first — all on the Windows PC)

**Step 1 — Run the WASAPI probe (no rebuild, ~2 minutes).**
```
cd windows-driver\tools
cl /EHsc /W3 wasapi_probe.cpp ole32.lib
wasapi_probe.exe
```
Interpret via the table in §4-#2. This single HRESULT selects the branch below.

**Step 2 — Full endpoint-state reset (most likely fix; no code change).**
1. Device Manager → Lampyris Virtual Microphone → Uninstall, check **"Delete the driver software for this device"**.
2. `pnputil /enum-drivers` → `pnputil /delete-driver oemNN.inf /uninstall /force` for **every** lampyris/sysvad-related package (audio INF, and any previously installed APO/Extension INFs).
3. Stop services: `net stop Audiosrv` and `net stop AudioEndpointBuilder`.
4. Under `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Capture`, delete the Lampyris endpoint GUID key(s) (identify by `FriendlyName` in `Properties`). Also remove stale devnodes under `HKLM\SYSTEM\CurrentControlSet\Enum\SWD\MMDEVAPI` referencing the Lampyris endpoint.
5. Reboot; install **only** the freshly built+signed `SignedPackage\ComponentizedAudioSample.inf`.
6. Verify the recreated endpoint key: `FxProperties` absent or empty; the DeviceFormat blob (`{f19f064d-…},0`) decodes to 2ch/48000/16; endpoint state ACTIVE. Re-run the probe / mictests.com.

**Step 3 — If still failing: trace the abort.**
- `wpr -start audio -filemode` → attempt the open → `wpr -stop audio.etl`; inspect `Microsoft-Windows-Audio` / AudioSes events for the failing stage (or use the Audio/Operational Event Log at verbose).
- Discriminator: does a pin-create `IRP_MJ_CREATE` reach the filter at all? (One temporary DbgPrint at the top of `LampyrisCreateClose` for non-control-device creates with a non-empty filename would answer this cheaply.) Never arrives → engine-side (#1/#2/#4). Arrives without `NewStream` → KS validation (#3) or dispatch (#5).

**Step 4 — Targeted fix per outcome.**
- FX residue found → strip the keys (or test with `PKEY_AudioEndpoint_Disable_SysFx = 1`) and keep APO INFs permanently out of the install.
- Cached-format mismatch → reset the DeviceFormat blob via reinstall (Step 2 covers it).
- Dispatch implicated → replace the manual hook with `PcDispatchIrp` forwarding.
- Structural dead end (only if the above all fail) → fallback bases exist: [JannesP/AudioMirror](https://github.com/JannesP/AudioMirror) (virtual mic, WaveRT), [VirtualDrivers/Virtual-Audio-Driver](https://github.com/VirtualDrivers/Virtual-Audio-Driver) (sysvad-derived, Win10/11), or a WaveCyclic design à la [Scream](https://github.com/duncanthrax/scream) — WO Mic itself ships a pre-WaveRT (WaveCyclic-era) `womic.sys`, and VAC deliberately keeps WaveCyclic as a compatibility fallback ([VAC docs](https://vac.muzychenko.net/en/description.htm)).

**Step 5 — Queued cleanups once capture works.**
Ring flush-on-stream-open + size/comment fix; delete the dead root driver project and stale `TestApp.cpp`/`IOCTL_LAMPYRIS_AUTHENTICATE`; single-source the IOCTL definitions; remove `LAMPYRIS-DEBUG` instrumentation (`grep -rn "LAMPYRIS-DEBUG" windows-driver/lampyris-sysvad/`).

---

## Appendix — key references

- WaveRT port driver open sequence: https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/understanding-the-wavert-port-driver
- `IMiniportWaveRTStreamNotification` (QI'd from the stream returned by NewStream): https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/portcls/nn-portcls-iminiportwavertstreamnotification
- RTAUDIO notification properties (all Pin-target): https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/ksproperty-rtaudio-buffer-with-notification , …/ksproperty-rtaudio-register-notification-event , …/ksproperty-rtaudio-query-notification-support
- `PKEY_AudioEndpoint_Supports_EventDriven_Mode`: https://learn.microsoft.com/en-us/windows/win32/coreaudio/pkey-audioendpoint-supports-eventdriven-mode
- WHQL audio requirements (EventDriven pairing; mode attribute must not be REQUIRED): https://learn.microsoft.com/en-us/previous-versions/windows/hardware/cert-program/deviceaudio-requirements
- AudioEndpointBuilder algorithm & persistent PropertyStore: https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/audio-endpoint-builder-algorithm
- Endpoint identity survives driver updates: https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/default-audio-endpoint-selection
- Shared-mode cached format: https://learn.microsoft.com/en-us/windows/win32/coreaudio/pkey-audioengine-deviceformat
- APO architecture (graph built at stream init; 10-failure auto-disable): https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/audio-processing-object-architecture
- Signal-processing modes (RAW or DEFAULT minimum): https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/audio-signal-processing-modes
- `IAudioClient::Initialize` HRESULTs: https://learn.microsoft.com/en-us/windows/win32/api/audioclient/nf-audioclient-iaudioclient-initialize
- KsCreatePin fails above the miniport (mode attribute): https://www.freelists.org/post/wdmaudiodev/KsCreatePin-Failure,1
- Full endpoint-registry reset procedure (MS engineer): https://www.freelists.org/post/wdmaudiodev/EXTERNAL-Re-Audio-driver-issues-No-speakers-or-headphones-are-plugged-in,2
- VAC pin-instance engine bug & architecture notes: https://vac.muzychenko.net/en/manual/winbugs.htm , https://vac.muzychenko.net/en/description.htm
- Chromium mic open path (`NotReadableError` mapping): https://chromium.googlesource.com/chromium/src/media/+/refs/heads/main/audio/win/audio_low_latency_input_win.cc
- Dispatch-hook anti-pattern (OSR): https://community.osr.com/t/sysdriver-how-to-capture-input-data-and-play-user-audio-application-data/56546
