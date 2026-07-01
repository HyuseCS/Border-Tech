# Start-Failure Bisect — Build & Capture Instructions (Windows PC)

**Root cause is confirmed:** the device **fails to start**. Device Manager →
Lampyris mic → **Events** tab shows `Problem: 0x15` (CM_PROB_FAILED_START) /
`Problem Status: 0xC00000BB` (**STATUS_NOT_SUPPORTED**). The PortCls audio
adapter never starts, so no KS capture filter exists — that's why ffmpeg tags
the mic `(none)` and fails to open it with `Unable to BindToObject`, why WASAPI
`NewStream` is never called, and why Voice Recorder sees no mic. (The IOCTL
producer still works because its control device is created in `DriverEntry`,
independent of the failing adapter.)

New DbgPrint instrumentation was added across the driver's **start /
registration path** so one DebugView capture names the exact call that returns
`0xC00000BB`. All added lines are tagged `// LAMPYRIS-DEBUG` (grep to remove
later).

## What was instrumented — START PATH (current focus, in call order)

| Stage | File | Print |
|------|------|-------|
| Adapter start steps | `adapter.cpp` `StartDevice` | `StartDevice: <AdapterCommon->Init \| PcRegisterAdapterPowerManagement \| InstallAllRenderFilters \| InstallAllCaptureFilters> -> 0x..` |
| Subdevice install | `common.cpp` `InstallSubdevice` | `InstallSubdevice <TopologyMicIn\|WaveMicIn>: <CreateAudioInterface \| PcNewPort \| MiniportCreate \| port->Init \| PcRegisterSubdevice> -> 0x..` |
| Bridge wiring | `common.cpp` `InstallEndpointFilters` | `InstallEndpointFilters: ConnectTopologies -> 0x..` |

## Also present — FORMAT PATH (from earlier; not the current failure)

These fire only *after* a successful start, so with the current start failure
they won't appear. Left in place; removed together at cleanup.

| Stage | File | Print |
|------|------|-------|
| Format probe | `EndpointsCommon/minwavert.cpp` `DataRangeIntersection` | `DRI: Pin=.. MyCh=.. ClientCh=.. -> NO_MATCH/PASS` |
| Stream create | `EndpointsCommon/minwavert.cpp` `NewStream` | `NewStream ENTER/EXIT: .. status=0x..` |
| Format check | `EndpointsCommon/minwavert.cpp` `IsFormatSupported` | `IsFormatSupported: Pin=.. req Nch/HzHz/Nbit/blkN -> 0x..` |
| DMA alloc | `EndpointsCommon/minwavertstream.cpp` `AllocateAudioBuffer` | `AllocBuffer: size=.. rate=..` |
| State change | `EndpointsCommon/minwavertstream.cpp` `SetState` | `SetState: Pin=.. X -> Y (0=STOP 1=ACQUIRE 2=PAUSE 3=RUN)` |

(Existing producer/drain prints also stay: `IOCTL_LAMPYRIS_PUSH_AUDIO`,
`ReadAudioData`, `TimerNotifyRT`, `GetPosition`.)

## 1. Build (Release — must match the sign script)

Open an **x64 WDK/EWDK Developer Command Prompt** and build the **whole
solution** — NOT the single `TabletAudioSample` project. Critical: the audio
negotiation/stream code (`minwavert.cpp`, `minwavertstream.cpp`) compiles into
the **`EndpointsCommon.lib`** static lib, and `TabletAudioSample.vcxproj`
references that lib only as a raw link input (no `<ProjectReference>`). Building
just `TabletAudioSample.vcxproj` relinks against a **stale** `EndpointsCommon.lib`
— any change in those two files is silently dropped from the .sys. (This exact
trap made the first instrumentation capture show only the `lampyris_core.cpp`
IOCTL prints and none of the open-path prints.)

**Build Release** (not Debug — `sign_driver.ps1` hardcodes the `x64\Release`
output dir):

```
cd windows-driver\lampyris-sysvad
msbuild sysvad.sln /p:Configuration=Release /p:Platform=x64
```

(Equivalent: build `EndpointsCommon\EndpointsCommon.vcxproj` first, then
`TabletAudioSample\TabletAudioSample.vcxproj`.)

Confirm a clean compile and that `EndpointsCommon.lib`'s timestamp updated.
(MSBuild does NOT auto-sign — the `<DriverSign>` block only sets the hash
algorithm; signing is the separate step below.)

## 1b. Sign (test certificate)

```
cd windows-driver
powershell -ExecutionPolicy Bypass -File sign_driver.ps1
```

This runs Inf2Cat → `sysvad.cat`, then signtool-signs **both** the catalog and
`lampyris-mic.sys` with the WDK test cert (thumb `DF5BB8…`) and verifies. Output
package: `lampyris-sysvad\TabletAudioSample\x64\SignedPackage\`
(`lampyris-mic.sys`, `ComponentizedAudioSample.inf`, `sysvad.cat`,
`lampyris-mic.cer`).

## 1c. Trust prerequisites on the target machine (one-time)

The install wizard rejects the package unless the test cert is trusted and test
signing is on. You've installed prior builds, so these are likely already set —
verify if a fresh machine:

```
bcdedit /set testsigning on            :: then reboot
certutil -addstore -f root             lampyris-mic.cer
certutil -addstore -f TrustedPublisher lampyris-mic.cer
```

## 2. Clean reinstall on the Windows test machine

Reinstall clean so you're testing the freshly built binary (and to clear the
stale MMDevice endpoint left by the failed-start device):

1. Device Manager → find **Lampyris Virtual Microphone** (Audio inputs and
   outputs, and/or Sound, video and game controllers) → Uninstall → check
   **"Delete the driver software for this device"**.
2. Install the freshly built test-signed driver (INF right-click → Install, or
   `pnputil /add-driver ... /install`).

Note: install itself runs `StartDevice`, so if DebugView is already capturing
(step 3) you may catch the failing prints during install — the disable→enable in
step 3 just gives you a clean, repeatable trigger.

## 3. Capture with DebugView — NEW OBJECTIVE: catch the device START

**Root cause is now known:** the device **fails to start** — Device Manager →
Lampyris mic → **Events** tab shows `Problem: 0x15` (CM_PROB_FAILED_START) with
`Problem Status: 0xC00000BB` (**STATUS_NOT_SUPPORTED**). Because the audio
adapter never starts, no KS capture filter exists — that's why ffmpeg reported
`Unable to BindToObject` and the mic tagged `(none)`, and why `NewStream` was
never called. The IOCTL producer still works because that control device is
created in `DriverEntry`, independent of the failing PortCls adapter.

The new start-path prints (tagged `[LAMPYRIS] StartDevice: ...` and
`[LAMPYRIS] InstallSubdevice <Name>: ...`) fire **at device start**, NOT at
mic-open. So we must be capturing when `StartDevice` runs.

1. Run Sysinternals **DebugView** (`Dbgview.exe`) **as Administrator**.
2. **Capture** menu → enable **Capture Kernel**, **Enable Verbose Kernel
   Output**, **Capture Events**.
3. **Trigger a fresh StartDevice** (do this with DebugView already capturing):
   Device Manager → find **Lampyris Virtual Microphone** → right-click →
   **Disable device**, then right-click → **Enable device**. (Enable re-runs
   `StartDevice`.) A reboot or uninstall+reinstall also works, but
   disable→enable is fastest.
   - You do **not** need the PC client, Android, or any capture app for this —
     we're catching the driver's own start sequence, not an audio open.
4. **File → Save** the log (send me the full file).

## 4. Read the log — decision tree

The prints run in this order; find the **last one that prints a success (`0x0`)
and the first that prints `0xC00000BB`** — the failing call is named right there.

Expected healthy prefix:
```
[LAMPYRIS] StartDevice: AdapterCommon->Init -> 0x0
[LAMPYRIS] StartDevice: PcRegisterAdapterPowerManagement -> 0x0
[LAMPYRIS] StartDevice: InstallAllRenderFilters -> 0x0
[LAMPYRIS] InstallSubdevice TopologyMicIn: CreateAudioInterface -> 0x0
[LAMPYRIS] InstallSubdevice TopologyMicIn: PcNewPort -> 0x0
[LAMPYRIS] InstallSubdevice TopologyMicIn: MiniportCreate -> 0x0
[LAMPYRIS] InstallSubdevice TopologyMicIn: port->Init -> 0x0
[LAMPYRIS] InstallSubdevice TopologyMicIn: PcRegisterSubdevice -> 0x0
[LAMPYRIS] InstallSubdevice WaveMicIn: CreateAudioInterface -> 0x0
[LAMPYRIS] InstallSubdevice WaveMicIn: PcNewPort -> 0x0
[LAMPYRIS] InstallSubdevice WaveMicIn: MiniportCreate -> 0x0
[LAMPYRIS] InstallSubdevice WaveMicIn: port->Init -> 0x0
[LAMPYRIS] InstallSubdevice WaveMicIn: PcRegisterSubdevice -> 0x0
[LAMPYRIS] InstallEndpointFilters: ConnectTopologies -> 0x0
[LAMPYRIS] StartDevice: InstallAllCaptureFilters -> 0x0
```

Whichever line first shows `-> 0xC00000BB` (or any non-zero) is the culprit:

- **`... port->Init -> 0xC00000BB`** → PortCls rejected the miniport/filter
  descriptor for that subdevice (Topology vs Wave tells us which). → next fix
  targets that descriptor.
- **`... PcRegisterSubdevice -> 0xC00000BB`** → name/interface registration
  mismatch (the `TopoName`/`WaveName` vs the INF `[Strings]` KSNAME_* entries).
- **`... MiniportCreate -> ...`** → the miniport create callback failed.
- **`ConnectTopologies -> ...`** → the wave⇔topology bridge-pin wiring is wrong.
- **`InstallAllCaptureFilters -> 0xC00000BB` but every `InstallSubdevice ...`
  line is `0x0`** → failure is in the capture-filter wrapper
  (`InstallEndpointCaptureFilters`), not the subdevice itself.

Send me the log and I'll make the targeted fix for the exact failing call.

## 5. Cleanup

After the failing step is confirmed, remove all scaffolding (format-path AND
start-path prints):

```
grep -rn "LAMPYRIS-DEBUG" windows-driver/lampyris-sysvad/
```

Delete those lines (or keep a minimal subset for future debugging).
