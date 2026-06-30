# Capture-Open Bisect — Build & Capture Instructions (Windows PC)

Temporary DbgPrint instrumentation was added to the driver to find **why the
Lampyris mic is silent** (green meter never moves, browsers throw
`NotReadableError`). The drain pipeline is proven correct; the failure is in
pin/format **negotiation** — the OS never drives the pin to `KSSTATE_RUN`. These
prints turn one DebugView capture into a bisect that names the failing step.

All added lines are tagged `// LAMPYRIS-DEBUG` (grep to remove later).

## What was instrumented (in call order)

| Stage | File | Print |
|------|------|-------|
| Format probe | `EndpointsCommon/minwavert.cpp` `DataRangeIntersection` | `DRI: Pin=.. MyCh=.. ClientCh=.. -> NO_MATCH/PASS` |
| Stream create | `EndpointsCommon/minwavert.cpp` `NewStream` | `NewStream ENTER: Pin=.. Capture=..` / `NewStream EXIT: .. status=0x..` |
| Format check | `EndpointsCommon/minwavert.cpp` `IsFormatSupported` | `IsFormatSupported: Pin=.. req Nch/HzHz/Nbit/blkN -> 0x..` |
| DMA alloc | `EndpointsCommon/minwavertstream.cpp` `AllocateAudioBuffer` | `AllocBuffer: size=.. rate=..` |
| State change | `EndpointsCommon/minwavertstream.cpp` `SetState` | `SetState: Pin=.. X -> Y (0=STOP 1=ACQUIRE 2=PAUSE 3=RUN)` |

(Existing producer/drain prints stay: `IOCTL_LAMPYRIS_PUSH_AUDIO`,
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

A stale cached endpoint default format can mask the real result, so reinstall
clean:

1. Device Manager → Sound, video and game controllers → **Lampyris Virtual
   Microphone** → Uninstall → check **"Delete the driver software for this
   device"**.
2. Install the freshly built test-signed driver (INF right-click → Install, or
   `pnputil /add-driver ... /install`).

## 3. Capture with DebugView

1. Run Sysinternals **DebugView** (`Dbgview.exe`) **as Administrator**.
2. **Capture** menu → enable **Capture Kernel**, **Enable Verbose Kernel
   Output**, **Capture Events**. (Same setup that produced
   `debug_logs/WINDOWSVM.log`; the kernel print filter is already non-zero on
   that machine.)
3. **Trigger an open:** Settings → System → Sound → the Lampyris mic →
   Properties → **"Listen to this device"** (and/or open `mictests.com`).
4. In parallel, start the PC client streaming from Android so the producer side
   is live (rules out "ring simply empty").
5. **File → Save** the log.

## 4. Read the log — decision tree

- **Only `DRI ... ClientCh=1 -> NO_MATCH`, no `NewStream ENTER`** → CONFIRMED:
  the stereo-only data range rejects mono consumers (the "Listen" meter, WebRTC,
  voice apps open mono). Negotiation dies before stream creation.
  → Fix: add a mono (1ch/48kHz/16-bit) data range + matching supported-format /
  mode entries, or relax the channel-exact intersection handler.

- **`NewStream ENTER` then `IsFormatSupported ... -> 0x...` (non-zero / NO_MATCH)**
  → the supported-formats table is too strict for the requested rate/bits.
  → Fix: widen `MicInPinSupportedDeviceFormats`.

- **`SetState ... -> 3` (RUN) appears, drain prints still absent / meter flat**
  → negotiation is fine; the bug is genuinely downstream in the WaveRT DMA /
  position path (`WriteBytes` / `UpdatePosition` / `GetReadPacket`). Re-scope.

## 5. Cleanup

After the failing step is confirmed, remove the scaffolding:

```
grep -rn "LAMPYRIS-DEBUG" windows-driver/lampyris-sysvad/EndpointsCommon/
```

Delete those lines (or keep a minimal subset for future debugging).
