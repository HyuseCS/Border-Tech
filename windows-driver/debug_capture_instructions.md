# Open-Path Debug — Build, Probe & Capture Instructions (Windows PC)

## Where we are

- **The device STARTS cleanly.** Start-path instrumentation shows every step
  returning `0x0` (adapter Init, power, both `InstallSubdevice TopologyMicIn` /
  `WaveMicIn`, `ConnectTopologies`, `InstallAllCaptureFilters`). The earlier
  `0xC00000BB` (STATUS_NOT_SUPPORTED) failed-start was a **stale binary** from a
  single-project build — the clean solution rebuild fixed it. Don't chase it.
- **The mic still won't open.** Even during a real WASAPI open ("Listen to this
  device"), the engine enumerates formats, accepts `2ch/48000/16bit -> 0x0`, and
  then **`NewStream` is never called** (no `DRI`, no `SetState`, no drain). The
  engine aborts the open *before* reaching our miniport.
- **ffmpeg / DirectShow is a RED HERRING.** `Unable to BindToObject` + `(none)`
  tag is just DirectShow's legacy KsProxy mishandling a WaveRT capture endpoint.
  The real clients (browser, Voice Recorder, WASAPI) use a different path. Stop
  using ffmpeg to test this.

**Confirmed root cause: a stale cached `PKEY_AudioEngine_DeviceFormat`** on the
capture endpoint. `GetMixFormat` on a capture endpoint reads that registry value
(`{F19F064D-082C-4E27-BC73-6882A1BB8E4C},0` under
`HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Capture\{guid}\Properties`)
and never asks the driver. `PKEY_AudioEngine_OEMFormat` only seeds it when the
property store is first created, so adding OEMFormat did not repair this
already-existing endpoint. Full write-up: `silence_debug_progress.md`
("Confirmed root cause"). The event-driven/pull-mode theory is now **ruled out**.

---

## STEP 0 (do this FIRST, before any capture attempt) — repair the endpoint

`windows-driver\reset_mic_endpoint.ps1` writes the correct stereo
`2ch / 48000 Hz / 16-bit` blob into the live endpoint's `DeviceFormat` and
`OEMFormat`. Run it from an **elevated** PowerShell prompt:

```
cd windows-driver
.\reset_mic_endpoint.ps1
```

What it does, in order: refuses to run unelevated -> matches the Lampyris capture
endpoint by name (or one of two documented fallback GUIDs) and **refuses to touch
anything if nothing matches** -> `reg export` backup of the endpoint's `Properties`
key -> prints the **BEFORE** decode -> writes the corrected blob -> prints the
**AFTER** decode -> restarts `AudioEndpointBuilder`/`Audiosrv` (this briefly cuts
all audio on the machine; it warns first).

**Record the BEFORE decode — it is the experiment.** Expect `DeviceFormat: <not set>`
or a mono / non-48000 / non-16-bit decode. If BEFORE already reads
`channels=2, rate=48000, bits=16, mask=0x3 (STEREO)`, the root cause is refuted on
this machine — stop and re-open the investigation.

`-Purge` (opt-in, never default) deletes the endpoint subtree instead, so Windows
recreates it fresh from the INF seed after a Device Manager disable/enable. A `.reg`
backup is still taken first; restore with `reg import <backup>.reg`.

---

## STEP 1 (after STEP 0) — WASAPI probe, prints the exact HRESULT (NO rebuild)

`windows-driver/tools/wasapi_probe.cpp` opens the Lampyris endpoint exactly like
a browser/Voice Recorder and prints the exact `AUDCLNT_*` HRESULT at each step.
It tries TWO ways — shared-mode WITHOUT the event flag, then WITH it. It's a plain
user-space program; the driver does not need rebuilding.

**The probe is now self-diagnosing.** For EVERY enumerated capture endpoint (not just
Lampyris) it prints, before it ever calls `GetMixFormat`:

- the endpoint id string,
- the raw hex + decoded `PKEY_AudioEngine_DeviceFormat`, and
- the raw hex + decoded `PKEY_AudioEngine_OEMFormat`

decoded as `channels=`, `rate=`, `bits=`, `blockAlign=`, `avgBytesPerSec=`,
`formatTag=`, `mask=`. A missing value prints `DeviceFormat: <not set>`. This means
the probe output alone tells you whether the cached format is stale — no extra
tooling, and healthy control endpoints appear side-by-side for comparison.

Run STEP 0 (`reset_mic_endpoint.ps1`) before this. After the repair the Lampyris
endpoint should read `channels=2, rate=48000, bits=16, mask=0x3 (STEREO)` and
`GetMixFormat -> S_OK` with `2 ch, 48000 Hz, 16 bit`.

In an **x64 Developer/WDK command prompt** (no admin needed):
```
cd windows-driver\tools
cl /EHsc /W3 wasapi_probe.cpp ole32.lib
wasapi_probe.exe
```
(Optional: have DebugView running to also see whether `NewStream` fires the
instant the probe attempts the open.)

Send me the full console output. Interpretation:

- **Attempt 1 (no event) SUCCEEDS, Attempt 2 (event) FAILS at `Initialize`**
  → confirms the **event-driven/notification mismatch**. Fix = either implement
  the WaveRT notification contract, or drop the event-driven opt-in from the INF
  (the `SYSVAD.I.TopologyMicIn.AddReg` / `WaveMicIn` EP lines in
  `ComponentizedAudioSample.inx`) so the engine uses the timer/polled model the
  driver already has.
- **`Initialize -> AUDCLNT_E_UNSUPPORTED_FORMAT`** → engine mix format vs pin
  format mismatch (channel mask). Fixable in the format table.
- **`Initialize -> AUDCLNT_E_ENDPOINT_CREATE_FAILED`** → PortCls can't build the
  capture pin graph — WaveRT pin/DMA config.
- **`>>> OPEN SUCCEEDED <<<`** → the driver side is fine; the problem is
  client/permission-side and we pivot entirely.

The failing HRESULT names the reason the engine bails; the fix follows from it.

---

## STEP 2 (only once a driver-side fix is identified) — build, sign, install

### 2a. Build (Release — must match the sign script)

Open an **x64 WDK/EWDK Developer Command Prompt** and build the **whole
solution** — NOT the single `TabletAudioSample` project. The
negotiation/stream code (`minwavert.cpp`, `minwavertstream.cpp`) compiles into
`EndpointsCommon.lib`, and `TabletAudioSample.vcxproj` references that lib only
as a raw link input (no `<ProjectReference>`). Building just the one project
relinks a **stale** `EndpointsCommon.lib` and silently drops those changes —
this is the exact trap that produced the phantom `0xC00000BB` failed-start.

```
cd windows-driver\lampyris-sysvad
msbuild sysvad.sln /p:Configuration=Release /p:Platform=x64
```

Confirm a clean compile and that `EndpointsCommon.lib`'s timestamp updated.
(MSBuild does NOT auto-sign — signing is the separate step below.)

### 2b. Sign (test certificate)

```
cd windows-driver
powershell -ExecutionPolicy Bypass -File sign_driver.ps1
```

Runs Inf2Cat → `sysvad.cat`, then signtool-signs the catalog and
`lampyris-mic.sys` with the WDK test cert (thumb `DF5BB8…`). Output package:
`lampyris-sysvad\TabletAudioSample\x64\SignedPackage\` (`lampyris-mic.sys`,
`ComponentizedAudioSample.inf`, `sysvad.cat`, `lampyris-mic.cer`).

### 2c. Trust prerequisites (one-time on a fresh machine)

```
bcdedit /set testsigning on            :: then reboot
certutil -addstore -f root             lampyris-mic.cer
certutil -addstore -f TrustedPublisher lampyris-mic.cer
```

### 2d. Clean reinstall

1. Device Manager → **Lampyris Virtual Microphone** (Audio inputs and outputs,
   and/or Sound video and game controllers) → Uninstall → check **"Delete the
   driver software for this device"**.
2. Install the freshly built package: right-click
   `SignedPackage\ComponentizedAudioSample.inf` → Install (or
   `pnputil /add-driver ... /install`). Ignore the APO and Extension INFs.

---

## STEP 3 — DebugView capture (to watch driver-side prints during a fix)

1. Run **DebugView** (`Dbgview.exe`) **as Administrator**.
2. **Capture** menu → **Capture Kernel**, **Enable Verbose Kernel Output**,
   **Capture Events**.
3. Trigger the thing you're testing:
   - **Start-path prints** (`StartDevice` / `InstallSubdevice`) fire at device
     start → Device Manager: **Disable** then **Enable** the Lampyris mic.
   - **Open-path prints** (`NewStream` / `DRI` / `SetState` / drain) fire at mic
     open → **Settings → Sound → Lampyris mic → Properties → "Listen to this
     device"** (or `mictests.com`).
4. **File → Save** the log and send it.

### Open-path decision tree
- **`NewStream ENTER` appears** → we're finally into the open path; the following
  prints (`AllocBuffer`, `SetState -> 3`, drain) show where it goes next.
- **Still no `NewStream`** → the engine aborts before reaching us — go back to
  the WASAPI probe HRESULT (Step 1) for the reason.

---

## Instrumentation reference (all tagged `// LAMPYRIS-DEBUG`)

Start path:
| File | Prints |
|------|--------|
| `adapter.cpp` `StartDevice` | `StartDevice: <Init \| PcRegisterAdapterPowerManagement \| InstallAllRenderFilters \| InstallAllCaptureFilters> -> 0x..` |
| `common.cpp` `InstallSubdevice` | `InstallSubdevice <TopologyMicIn\|WaveMicIn>: <CreateAudioInterface \| PcNewPort \| MiniportCreate \| port->Init \| PcRegisterSubdevice> -> 0x..` |
| `common.cpp` `InstallEndpointFilters` | `InstallEndpointFilters: ConnectTopologies -> 0x..` |

Open / format path:
| File | Prints |
|------|--------|
| `EndpointsCommon/minwavert.cpp` `DataRangeIntersection` | `DRI: Pin=.. MyCh=.. ClientCh=.. -> NO_MATCH/PASS` |
| `EndpointsCommon/minwavert.cpp` `NewStream` | `NewStream ENTER/EXIT: .. status=0x..` |
| `EndpointsCommon/minwavert.cpp` `IsFormatSupported` | `IsFormatSupported: Pin=.. req Nch/HzHz/Nbit/blkN -> 0x..` |
| `EndpointsCommon/minwavertstream.cpp` `AllocateAudioBuffer` | `AllocBuffer: size=.. rate=..` |
| `EndpointsCommon/minwavertstream.cpp` `SetState` | `SetState: Pin=.. X -> Y (0=STOP 1=ACQUIRE 2=PAUSE 3=RUN)` |

(Producer/drain prints also present: `IOCTL_LAMPYRIS_PUSH_AUDIO`,
`ReadAudioData`, `TimerNotifyRT`, `GetPosition`.)

## Cleanup (once the bug is fixed)

```
grep -rn "LAMPYRIS-DEBUG" windows-driver/lampyris-sysvad/
```
Delete those lines (or keep a minimal subset). `windows-driver/tools/wasapi_probe.cpp`
is a standalone diagnostic — delete or keep as you like.
