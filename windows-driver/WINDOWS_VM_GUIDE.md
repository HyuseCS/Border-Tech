# Lampyris Mic — Windows VM Guide

This is the single document to follow while sitting at the Windows VM.
It assumes you remember nothing. Every command is copy-pasteable.

**Notation used in this file**

- Every command block says which folder to run it in and whether it needs
  Administrator rights.
- Lines marked **(not verified in-repo — confirm on first run)** are best-guess
  steps that are NOT confirmed by a file in this repository. Treat them as a
  starting point, not as fact. Everything else is taken from a real file in the
  repo (script, project file, or debug notes).

**Your repo drive letter**

The repo lives on a mapped/shared drive on the VM. The last saved probe output
(`debug_logs/wasapi_probe.txt`) shows the prompt `Z:\windows-driver\tools>`, so
the repo was at `Z:\` on that run. But `trust_certs.ps1` and `export_cert.ps1`
have `D:\Border-Tech\...` hardcoded. These two disagree.

Do this first, in PowerShell:

```powershell
Get-PSDrive -PSProvider FileSystem | Select-Object Name, Root, Used
```

Find the drive that holds the repo. In this whole guide, replace `Z:` with your
real drive letter wherever it appears.

---

**This file supersedes `testing_instructions.md`.**

`testing_instructions.md` at the repo root is an older, shorter manual-test
note. Its driver-install steps and its reboot-and-test steps are correct and are
reproduced here (sections 6.2 and 9). But it also tells you to copy the
`x64\Release\` folder into the VM, and to cross-compile with the
`x86_64-pc-windows-gnu` target. This guide uses `x64\SignedPackage\` and the
MSVC target instead, and explains why (sections 6.1 and 7). **Follow this file,
not that one.**

---

## 1. TL;DR — the 5-minute test (NO build needed)

Read this section first. It tests the confirmed root cause and does not need any
compiler, WDK, or Rust. If it works, you are done for today.

The confirmed root cause (from `silence_debug_progress.md`): Windows caches the
mic's audio format in the registry. That cached value is stale. Windows reads the
cache and never asks the driver, so it gives up before the driver is ever called.
The script below rewrites the cached value.

### Step 1 — open an ELEVATED PowerShell

Press Start, type `PowerShell`, right-click **Windows PowerShell**, choose
**Run as administrator**.

### Step 2 — run the reset script (needs Administrator)

Working directory: `Z:\windows-driver`

```powershell
cd Z:\windows-driver
.\reset_mic_endpoint.ps1
```

If PowerShell blocks the script, run this instead (same folder, same elevation):

```powershell
powershell -ExecutionPolicy Bypass -File .\reset_mic_endpoint.ps1
```

### Step 3 — read the BEFORE line. This is the experiment.

The script prints a block like this (text taken from the script's own
`Write-Host` lines):

```
Matched 1 Lampyris capture endpoint(s):
  {........-....-....-....-............}  [friendly name 'External Microphone Headphone (Lampyris Virtual Microphone)']

=== Endpoint {....} — External Microphone Headphone (Lampyris Virtual Microphone) ===
  Backing up HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Capture\{....}\Properties
          -> Z:\windows-driver\mic_endpoint_backup_...._........-.......reg

  --- BEFORE ---
  DeviceFormat: <not set>
  OEMFormat raw (48 bytes): 41 00 00 00 ...
  OEMFormat: channels=2, rate=48000, bits=16, blockAlign=4, avgBytesPerSec=192000, formatTag=0xFFFE, mask=0x3 (STEREO)
```

**Now decide. Look ONLY at the `DeviceFormat:` line under `--- BEFORE ---`.**

| BEFORE `DeviceFormat` shows | Meaning | What to do |
|---|---|---|
| `<not set>` | Root cause CONFIRMED | Continue to Step 4 |
| `channels=1` (mono), or `rate=` not 48000, or `bits=` not 16 | Root cause CONFIRMED | Continue to Step 4 |
| `channels=2, rate=48000, bits=16, mask=0x3 (STEREO)` | Root cause **REFUTED** on this machine | **STOP.** Do not run anything else. Save the whole console output and report back. |

The refuted case matters. If the cached value is already correct, the theory is
wrong and continuing wastes your time.

### Step 4 — let the script finish

After the BEFORE block, the script prints `--- AFTER ---` with the corrected
values, then a warning and a pause:

```
*** WARNING ***
About to restart AudioEndpointBuilder and Audiosrv. This briefly interrupts ALL
audio on this machine — music, calls, other microphones will cut out for a few
seconds. This is expected and harmless, but close anything mid-call first.
```

It waits 3 seconds, restarts the audio services, then prints:

```
Audiosrv status: Running

Done. Next: run windows-driver\tools\wasapi_probe.exe and check that
GetMixFormat returns S_OK with 2 ch, 48000 Hz, 16 bit for the Lampyris endpoint.
```

Expect all audio on the machine to cut out for a few seconds. That is normal.

### Step 5 — run the probe

Working directory: `Z:\windows-driver\tools`. No Administrator needed.

```
cd Z:\windows-driver\tools
wasapi_probe.exe
```

If `wasapi_probe.exe` is missing or you want a fresh build, see section 8.

**What "fixed" looks like** — the Lampyris entry must show:

```
[1] External Microphone Headphone (Lampyris Virtual Microphone)
      id: {0.0.1.00000000}.{........-....-....-....-............}
      DeviceFormat raw (48 bytes): 41 00 00 00 ...
      DeviceFormat: channels=2, rate=48000, bits=16, blockAlign=4, avgBytesPerSec=192000, formatTag=0xFFFE, mask=0x3 (STEREO), validBits=16
      OEMFormat: channels=2, rate=48000, bits=16, ...
  Activate(IAudioClient)             -> 0x00000000  S_OK
  GetMixFormat                       -> 0x00000000  S_OK
      fmt: 2 ch, 48000 Hz, 16 bit, tag 0xFFFE
  Initialize(SHARED, mix fmt)        -> 0x00000000  S_OK
      (open OK — endpoint healthy)
```

**What "still broken" looks like** — this is the real saved output from before
the fix (`debug_logs/wasapi_probe.txt`):

```
[1] External Microphone Headphone (Lampyris Virtual Microphone)
  Activate(IAudioClient)             -> 0x00000000  S_OK
  GetMixFormat                       -> 0x88890008  AUDCLNT_E_UNSUPPORTED_FORMAT
```

Note: that saved file has no `id:` / `DeviceFormat:` / `OEMFormat:` lines,
because it was captured before the probe was updated to print them. A fresh
build of the probe will print them.

If `GetMixFormat` still returns `0x88890008` after the reset, go to section 10
(Troubleshooting), row "GetMixFormat still fails".

### Step 6 — real audio test

Start the Android app, start the PC client, then open Voice Recorder or a browser
mic test page. See section 9 for the full procedure. **Audible sound with a
moving level meter is the acceptance bar.** Nothing counts as fixed until that
happens.

---

## 2. One-time VM setup

Do this once per VM. Skip anything already installed.

### 2.1 Visual Studio 2022 + Desktop C++ workload

Needed for: `msbuild`, `cl.exe`, the x64 Native Tools command prompt.

Install Visual Studio 2022 (Community is fine) and tick the workload
**Desktop development with C++**.
**(not verified in-repo — confirm on first run)**

Verify: press Start and type `x64 Native Tools`. You should see
**x64 Native Tools Command Prompt for VS 2022**. Open it and run:

```
cl
```

Expected: a version banner starting `Microsoft (R) C/C++ Optimizing Compiler
Version 19.xx...`. If you get "'cl' is not recognized", you opened the wrong
prompt.

### 2.2 Windows Driver Kit (WDK) + matching SDK

Needed for: building the kernel driver, `Inf2Cat.exe`, `signtool.exe`.

Install the WDK for Windows 11 and the matching Windows SDK. The WDK installer
also adds the Visual Studio driver project templates.
**(not verified in-repo — confirm on first run)**

Verify which WDK versions you have (any command prompt, no admin):

```
dir "C:\Program Files (x86)\Windows Kits\10\bin"
```

Expected: a list of version folders, for example `10.0.22621.0`,
`10.0.26100.0`.

**Important.** `sign_driver.ps1` hardcodes this exact path:

```
C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0
```

If `10.0.22621.0` is NOT in the `dir` listing above, the sign script will fail
with `Inf2Cat not found at: ...`. Fix: open `Z:\windows-driver\sign_driver.ps1`
and change the `$WdkBinRoot` line (near the top, in the `--- Configuration ---`
block) to a version folder you actually have.

### 2.3 Sysinternals DebugView

Needed for: seeing the driver's own `[LAMPYRIS]` / `LAMPYRIS-DEBUG` prints.

Download DebugView from Microsoft Sysinternals and unzip it anywhere.
**(not verified in-repo — confirm on first run)**

It must be run **as Administrator**. See section 3d for the exact settings.

### 2.4 Rust, MSVC toolchain

Needed for: building the PC client (Lampyris).

Install Rust from rustup.rs. Choose the default host triple
`x86_64-pc-windows-msvc`. **(not verified in-repo — confirm on first run)**

Verify (any command prompt):

```
rustc --version
cargo --version
rustup show
```

Expected: version numbers, and `rustup show` lists
`x86_64-pc-windows-msvc` as the default host.

`pc-client/.cargo/config.toml` sets `-C target-feature=+crt-static` for that
target, so the built exe does not need the Visual C++ redistributable.

### 2.5 adb (Android platform-tools)

Needed for: USB mode. The PC client shells out to `adb forward` when USB mode is
active (`pc-client/src/app_state.rs`).

Download Android SDK platform-tools and put the folder on your `PATH`.
**(not verified in-repo — confirm on first run)**

Verify:

```
adb version
```

### 2.6 Turn on test signing and REBOOT

The driver is signed with a self-made test certificate. Windows will refuse to
load it unless test signing is on.

Elevated command prompt or elevated PowerShell:

```
bcdedit /set testsigning on
```

Expected output: `The operation completed successfully.`

**Now reboot the VM.** The setting only takes effect after a restart.

After reboot, check the desktop bottom-right corner. It should say
**Test Mode** with a Windows build number. That is the sign it worked.

Verify from a command prompt:

```
bcdedit /enum {current}
```

Expected: a line reading `testsigning             Yes`.

### 2.7 Trust the test certificate

The certificate file is produced by the signing step. It lives at:

```
Z:\windows-driver\lampyris-sysvad\TabletAudioSample\x64\SignedPackage\lampyris-mic.cer
```

Elevated PowerShell, working directory `Z:\windows-driver`:

```powershell
cd Z:\windows-driver
.\trust_certs.ps1
```

This script does three things: turns on test signing, adds the `.cer` to the
**root** store, and adds it to the **TrustedPublisher** store. Both stores are
required — root makes Windows trust the signature, TrustedPublisher stops the
"do you want to install this driver" prompt from blocking.

Expected output from `certutil`: two blocks each ending with
`CertUtil: -addstore command completed successfully.`

If the script says the `.cer` was not found, you have not run `sign_driver.ps1`
yet. Do section 5 first, then come back.

### 2.8 Check the signing certificate thumbprint

`sign_driver.ps1` hardcodes the certificate thumbprint
`DF5BB8BF921D9CFCF15636514913EFF63FFC257D`. If your VM has a different WDK test
certificate, signing will fail.

Any PowerShell, no admin needed, working directory `Z:\windows-driver`:

```powershell
cd Z:\windows-driver
.\find_cert.ps1
```

This prints the `Subject` and `Thumbprint` of every `WDKTestCert*` certificate in
your personal certificate store. Expected shape:

```
Subject    : CN=WDKTestCert <user>,131...
Thumbprint : DF5BB8BF921D9CFCF15636514913EFF63FFC257D
```

If the printed thumbprint does NOT match `DF5BB8BF...`, edit the `$CertThumb`
line in `sign_driver.ps1` to the value you just saw. Also edit `$thumb` in
`export_cert.ps1` if you plan to use it.

If `find_cert.ps1` prints nothing at all, you have no WDK test certificate yet.
Visual Studio creates one the first time you build a driver project with test
signing enabled in the project settings.
**(not verified in-repo — confirm on first run)**

---

## 3. The fast path (no rebuild) — full detail

This is steps (a)–(d) from the runbook in `silence_debug_progress.md`. None of
them needs a compiler. Do them in order. Do not reorder.

### 3a. Run the reset script

Working directory: `Z:\windows-driver`. **Administrator required.**

```powershell
cd Z:\windows-driver
.\reset_mic_endpoint.ps1
```

What the script does, in order (from its own source):

1. Refuses to run unless elevated. Unelevated it prints in red
   `ERROR: this script must be run from an ELEVATED PowerShell prompt.` and
   exits with code 1.
2. Scans
   `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Capture`
   and matches an endpoint ONLY if its friendly name contains `Lampyris`, or its
   GUID is one of two documented fallback GUIDs. Nothing else is touched.
3. If nothing matched, it prints
   `ERROR: no Lampyris capture endpoint found.` plus
   `Nothing was written. No registry key was modified.` and exits with code 2.
4. Exports a `.reg` backup of the endpoint's `Properties` key into
   `Z:\windows-driver\mic_endpoint_backup_<guid>_<timestamp>.reg`. If the backup
   fails it prints `ERROR: registry backup failed. Refusing to modify anything.`
   and exits with code 3.
5. Prints the `--- BEFORE ---` decode.
6. Writes the correct 48-byte stereo blob to both `DeviceFormat` and
   `OEMFormat` as `REG_BINARY`. This is idempotent — running it twice is safe.
7. Prints the `--- AFTER ---` decode.
8. Warns, waits 3 seconds, then restarts `AudioEndpointBuilder` (which cascades
   to `Audiosrv`), and prints `Audiosrv status: Running`.

**Expected AFTER decode:**

```
  --- AFTER ---
  DeviceFormat raw (48 bytes): 41 00 00 00 28 00 00 00 FE FF 02 00 80 BB 00 00 ...
  DeviceFormat: channels=2, rate=48000, bits=16, blockAlign=4, avgBytesPerSec=192000, formatTag=0xFFFE, mask=0x3 (STEREO)
  OEMFormat: channels=2, rate=48000, bits=16, blockAlign=4, avgBytesPerSec=192000, formatTag=0xFFFE, mask=0x3 (STEREO)
  Backup: Z:\windows-driver\mic_endpoint_backup_..._....reg   (restore with: reg import "...")
```

**If this instead happens:**

| You see | Meaning | Do this |
|---|---|---|
| `ERROR: this script must be run from an ELEVATED PowerShell prompt.` | Not admin | Close it. Right-click PowerShell, Run as administrator, retry. |
| `ERROR: no Lampyris capture endpoint found.` | The mic device is not installed or is disabled | Open Device Manager, look for **Lampyris Virtual Microphone** under "Audio inputs and outputs". Enable it, or install it (section 6). |
| `ERROR: registry backup failed.` | `reg.exe export` failed | Confirm you really are elevated and the folder is writable. Nothing was changed. |
| BEFORE already shows `channels=2, rate=48000, bits=16, mask=0x3 (STEREO)` | Theory refuted | STOP. Do not do 3b–3e. Save the output and report back. |

**The `-Purge` switch.** There is a second, more destructive mode:

```powershell
cd Z:\windows-driver
.\reset_mic_endpoint.ps1 -Purge
```

Instead of writing corrected values, it DELETES the whole endpoint registry
subtree so Windows recreates it fresh from the INF. It still takes a `.reg`
backup first. It then prints:

```
  -Purge specified: deleting the endpoint registry subtree.
  Deleted. Next steps:
    1. Open Device Manager.
    2. Find the Lampyris Mic device, Disable it, then Enable it again.
    3. Windows recreates the endpoint fresh, seeded from the INF OEMFormat value.
    4. Re-run this script WITHOUT -Purge to confirm the new DeviceFormat decode.
```

`-Purge` is opt-in and never the default. Use it only when the plain run did not
help, or when you want to prove the INF seeds the value correctly on a fresh
endpoint.

### 3b. Re-run the probe

Working directory: `Z:\windows-driver\tools`. No admin needed.

```
cd Z:\windows-driver\tools
wasapi_probe.exe
```

**Expected:** `GetMixFormat -> 0x00000000  S_OK` with `fmt: 2 ch, 48000 Hz, 16
bit, tag 0xFFFE`, and `Initialize(SHARED, mix fmt) -> 0x00000000  S_OK` followed
by `(open OK — endpoint healthy)`.

The probe then runs a deep probe on the Lampyris endpoint:

```
=== Lampyris deep probe (explicit 2ch/48000/16 WAVEFORMATEXTENSIBLE) ===

  Activate(IAudioClient)             -> 0x00000000  S_OK
  IsFormatSupported(SHARED, dev fmt) -> 0x00000000  S_OK
  IsFormatSupported(EXCL, dev fmt)   -> 0x00000000  S_OK

-- Attempt: SHARED, explicit dev fmt, no event --
  Activate(IAudioClient)             -> 0x00000000  S_OK
  Initialize                         -> 0x00000000  S_OK
      >>> SHARED OPEN SUCCEEDED <<<

-- Attempt: SHARED + EVENTCALLBACK, explicit dev fmt --
  Activate(IAudioClient)             -> 0x00000000  S_OK
  Initialize(EVENTCALLBACK)          -> 0x00000000  S_OK
      >>> EVENT-DRIVEN OPEN SUCCEEDED <<<

-- Attempt: EXCLUSIVE, explicit dev fmt (bypasses engine pipe; watch for NewStream) --
  Activate(IAudioClient)             -> 0x00000000  S_OK
  Initialize(EXCLUSIVE)              -> 0x00000000  S_OK
      buffer frames: <number>
      >>> EXCLUSIVE OPEN SUCCEEDED — driver-side pin creation WORKS <<<
```

The probe always ends with its own interpretation key:

```
Done. Interpretation:
  control mic also fails            -> VM audio engine broken, not our driver
  EXCLUSIVE succeeds, SHARED fails  -> driver fine; shared-mode engine pipe is the bug
  EXCLUSIVE fails too               -> the HRESULT + DebugView (NewStream?) name the KS-level reason
```

**If this instead happens:**

| You see | Meaning | Do this |
|---|---|---|
| `GetMixFormat -> 0x88890008  AUDCLNT_E_UNSUPPORTED_FORMAT` | The registry write in 3a did not take effect | Confirm 3a was elevated and printed a good `--- AFTER ---`. Confirm `Audiosrv status: Running`. Re-run 3a, then reboot the VM and re-run the probe. |
| `No capture endpoint with 'Lampyris' in the name.` (exit code 2) | The device is missing or disabled | Device Manager → enable or install the device (section 6). |
| `Active capture endpoints: 0` | No mics at all in this VM | Check the VM's audio device settings. |
| Endpoint `[0]` (the normal mic) ALSO fails | The VM audio engine itself is broken, not the driver | Fix the VM audio first. This is what the probe's own interpretation key says. |

### 3c. Watch DebugView during a real mic open

1. Run `Dbgview.exe` **as Administrator**.
2. **Capture** menu → tick **Capture Kernel**, **Enable Verbose Kernel Output**,
   and **Capture Events**.
3. Now trigger a real microphone open. Either:
   - Settings → Sound → Lampyris mic → Properties → **Listen to this device**, or
   - open a browser mic test page, or
   - open Voice Recorder.

**Expected:** lines containing `NewStream`, `AllocBuffer` and `SetState` appear
for the first time. Specifically the driver prints:

```
NewStream ENTER: ... 
AllocBuffer: size=.. rate=..
SetState: Pin=.. X -> Y (0=STOP 1=ACQUIRE 2=PAUSE 3=RUN)
NewStream EXIT: .. status=0x..
```

**Which prints fire when — this matters.**

- **Start-path prints** (`StartDevice`, `InstallSubdevice`, `ConnectTopologies`)
  fire when the DEVICE STARTS: at boot, at install, or when you Disable then
  Enable the device in Device Manager. Have DebugView running BEFORE you enable.
- **Open-path prints** (`NewStream`, `DRI`, `IsFormatSupported`, `SetState`)
  fire when an APP OPENS THE MIC. Have DebugView running before you press
  "Listen to this device".

**If `NewStream` still never appears:** the engine is still giving up before it
reaches the driver. Save the full `wasapi_probe.exe` output and the DebugView log
(File → Save) and treat it as a new investigation. Do not guess.

### 3d. End-to-end audio test

See section 9. This is the acceptance bar.

**If audio is silent but 3c passed** (`NewStream` did appear): that is a
different, narrower bug in the ring-buffer / `ReadAudioData` consumption path in
`minwavertstream.cpp`. Not this one. Note it and report.

---

## 4. Compiling the driver

Do this only after the fast path (section 3) passes, or when you have changed
driver source. It makes the fix durable for clean installs.

### 4.1 Open the right command prompt

Press Start and open **x64 Native Tools Command Prompt for VS 2022**.

If you installed the EWDK instead of the WDK, open the EWDK build environment
prompt instead. **(not verified in-repo — confirm on first run)**

Do NOT use a plain `cmd.exe` or a plain PowerShell window. `msbuild` will not be
on the PATH there.

Verify:

```
msbuild -version
```

Expected: `MSBuild version 17.x.x for .NET Framework` and a version number.

### 4.2 HARD RULE: always build the WHOLE SOLUTION

**Never build the `TabletAudioSample` project alone.**

Why: the stream and format-negotiation code (`minwavert.cpp`,
`minwavertstream.cpp`) compiles into `EndpointsCommon.lib`.
`TabletAudioSample.vcxproj` pulls that library in only as a raw link input
(`AdditionalDependencies` → `.\..\EndpointsCommon\$(IntDir)\EndpointsCommon.lib`).
There is NO `<ProjectReference>` to `EndpointsCommon`. So MSBuild does not know
it must rebuild that library first. A single-project build silently relinks a
STALE `EndpointsCommon.lib` and quietly throws away your changes.

This exact trap produced a fake `0xC00000BB` (`STATUS_NOT_SUPPORTED`) device
start failure and cost a multi-day false lead. That failure was never a real bug.
Do not chase it. Build the solution.

### 4.3 The build command

Working directory: `Z:\windows-driver\lampyris-sysvad`. Admin not required.

```
cd /d Z:\windows-driver\lampyris-sysvad
msbuild sysvad.sln /p:Configuration=Release /p:Platform=x64
```

**Build Release.** `sign_driver.ps1` hardcodes the path `x64\Release`. A Debug
build will not be found by the signing step.

For a guaranteed-clean rebuild (recommended when chasing this bug), add a clean
pass first:

```
cd /d Z:\windows-driver\lampyris-sysvad
msbuild sysvad.sln /t:Clean /p:Configuration=Release /p:Platform=x64
msbuild sysvad.sln /p:Configuration=Release /p:Platform=x64
```

**How long it takes:** a few minutes on a VM.
**(not verified in-repo — confirm on first run)**

### 4.4 What is in the solution

`sysvad.sln` contains these buildable projects:

| Project | Needed for the mic? |
|---|---|
| `package` (`Package\package.VcxProj`) | no |
| `KeywordDetectorContosoAdapter` | no |
| `EndpointsCommon` | **YES** — the stream/format code |
| `TabletAudioSample` | **YES** — produces `lampyris-mic.sys` |
| `DelayAPO` (`APO\DelayAPO`) | no |
| `SwapAPO` (`APO\SwapAPO`) | no |
| `KwsAPO` (`APO\KWSApo`) | no |
| `AecAPO` (`APO\AecApo`) | no |

### 4.5 The APO failure is EXPECTED — do not panic

**The four APO subprojects (DelayAPO, SwapAPO, KwsAPO, AecAPO) FAIL to build.**
This is known and recorded in `silence_debug_progress.md`. They are NOT needed to
produce `lampyris-mic.sys`.

So you will see red error lines at the end of the build. That is normal.

**How to tell a real success from a real failure:** ignore the summary line and
check that the driver binary was actually produced and its timestamp is new:

```
dir Z:\windows-driver\lampyris-sysvad\TabletAudioSample\x64\Release\lampyris-mic.sys
dir Z:\windows-driver\lampyris-sysvad\EndpointsCommon\x64\Release\EndpointsCommon.lib
```

Both must exist and both timestamps must be from the build you just ran. If
`EndpointsCommon.lib`'s timestamp is old, your changes did NOT make it into the
driver — you hit the stale-lib trap. Rebuild the whole solution.

The exact path of `EndpointsCommon.lib` depends on the project's intermediate
directory (`$(IntDir)`), so if the `dir` above finds nothing, search for it:

```
dir /s /b Z:\windows-driver\lampyris-sysvad\EndpointsCommon.lib
```

**(the exact `EndpointsCommon.lib` output folder is not verified in-repo —
confirm on first run)**

### 4.6 Where the outputs land

Working folder `Z:\windows-driver\lampyris-sysvad\TabletAudioSample\x64\Release\`
contains (confirmed by the equivalent `x64\Debug\` folder in the repo):

- `lampyris-mic.sys` — the driver binary
- `lampyris-mic.pdb` — debug symbols
- `lampyris-mic.cer` — the test certificate
- `ComponentizedAudioSample.inf` — the mic install file (**this is the one you install**)
- `ComponentizedApoSample.inf` — APO install file, **not needed**
- `ComponentizedAudioSampleExtension.inf` — extension INF, **not needed**

MSBuild does NOT sign anything. Signing is section 5.

**Do not install straight from this `Release\` folder.** It has no signed
catalog. `sign_driver.ps1` copies three of these files into a separate
`x64\SignedPackage\` folder, adds a signed `sysvad.cat`, and signs the `.sys`.
That `SignedPackage\` folder is what you install. See section 6.1.

---

## 5. Signing

### 5.1 Run the sign script

Working directory: `Z:\windows-driver`. Admin not strictly required for signing,
but run elevated to be safe. **(elevation requirement not verified in-repo)**

```
cd /d Z:\windows-driver
powershell -ExecutionPolicy Bypass -File sign_driver.ps1
```

### 5.2 What it does — 5 stages

The script prints a banner then works through numbered stages (its own labels):

**`[0/4] Preparing staging directory...`**
Deletes and recreates
`lampyris-sysvad\TabletAudioSample\x64\SignedPackage\`, then copies in exactly
three files from `x64\Release`: `lampyris-mic.sys`,
`ComponentizedAudioSample.inf`, `lampyris-mic.cer`. A clean folder is required
because `Inf2Cat` processes every `.inf` it finds in a directory.

Prints:
```
  -> Staged: lampyris-mic.sys, ComponentizedAudioSample.inf, lampyris-mic.cer
```

**`[1/4] Generating catalog file (sysvad.cat) with Inf2Cat...`**
Runs `Inf2Cat /driver:"<StageDir>" /os:10_x64 /verbose`.

Prints on success:
```
  -> sysvad.cat created successfully.
```

**`[2/4] Signing sysvad.cat with test certificate...`**
Runs `signtool sign /v /s My /sha1 <thumbprint> /fd sha256 /t
http://timestamp.digicert.com sysvad.cat`. If the timestamp server is
unreachable (very likely on an offline VM) it prints
`  -> Timestamping server unreachable, signing without timestamp...` and retries
without `/t`. That is fine.

Prints on success:
```
  -> sysvad.cat signed successfully.
```

**`[3/4] Signing lampyris-mic.sys with test certificate...`**
Same signing logic applied to the `.sys` file.

Prints on success:
```
  -> lampyris-mic.sys signed successfully.
```

**`[4/4] Verifying signatures...`**
Runs `signtool verify /v /pa` on both files and prints their output under
`--- sysvad.cat ---` and `--- lampyris-mic.sys ---`.

### 5.3 What success looks like

The script ends with:

```
============================================
 Done! Signed driver package ready.
============================================

Signed package location:
  Z:\windows-driver\lampyris-sysvad\TabletAudioSample\x64\SignedPackage

Files to copy to the VM:
  - lampyris-mic.sys  (signed driver)
  - ComponentizedAudioSample.inf  (install config)
  - sysvad.cat  (signed catalog)
  - lampyris-mic.cer  (test certificate)
```

### 5.4 The two failure modes

**Failure A — wrong WDK path.**

```
Inf2Cat not found at: C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x86\Inf2Cat.exe
```
or
```
SignTool not found at: C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\signtool.exe
```

Fix: list your installed WDK versions:

```
dir "C:\Program Files (x86)\Windows Kits\10\bin"
```

Then edit `Z:\windows-driver\sign_driver.ps1` and change the `$WdkBinRoot` line
to a version folder that actually exists on your machine.

**Failure B — wrong certificate thumbprint.**

The script will fail during stage `[2/4]` or `[3/4]` with
`SignTool failed to sign sysvad.cat` or
`SignTool failed to sign lampyris-mic.sys`. `signtool` itself typically reports
that no certificate matched the given SHA1.
**(exact signtool error text not verified in-repo)**

Fix: print your real thumbprint:

```powershell
cd Z:\windows-driver
.\find_cert.ps1
```

Then edit the `$CertThumb` line in `sign_driver.ps1` to the value printed.

**Third possible failure — missing Release build.**

```
Driver release directory not found at: Z:\windows-driver\lampyris-sysvad\TabletAudioSample\x64\Release
```

Fix: you built Debug, or you did not build at all. Go back to section 4 and build
`/p:Configuration=Release /p:Platform=x64`.

### 5.5 Re-exporting the certificate

If `lampyris-mic.cer` is missing or invalid, `export_cert.ps1` re-exports it from
your certificate store. **Warning:** that script also hardcodes both the
thumbprint AND an output path of `D:\Border-Tech\...`. Edit both before running
it if your repo is not on `D:`.

---

## 6. Installing, reinstalling, and fully uninstalling the driver

### 6.1 Which folder do I install from? Release vs SignedPackage

There are two folders and they are not the same. Use the right one.

| Folder | What it is | Install from it? |
|---|---|---|
| `lampyris-sysvad\TabletAudioSample\x64\Release\` | Raw MSBuild output. Has `lampyris-mic.sys`, three `.inf` files, `.pdb`, `.cer`. **No signed catalog (`sysvad.cat`).** | **No** — not normally |
| `lampyris-sysvad\TabletAudioSample\x64\SignedPackage\` | Staged and signed package produced by `sign_driver.ps1`. Has exactly four files: `lampyris-mic.sys` (signed), `ComponentizedAudioSample.inf`, `sysvad.cat` (signed catalog), `lampyris-mic.cer`. | **Yes — install from here** |

**Rule: install from `SignedPackage\`.** It is the only folder with a signed
catalog, which is what makes Windows accept the driver in test-signing mode.
`sign_driver.ps1` builds it by copying three files out of `Release\`, running
`Inf2Cat` to produce `sysvad.cat`, and signing both the catalog and the `.sys`.

**Note on `testing_instructions.md`.** That older note tells you to copy the
`x64\Release\` folder into the VM (its section 3). That works only if you sign
in place or if signature checks happen to pass. Prefer `SignedPackage\`. If you
do follow the older note, understand you are installing an unsigned build.

So the file you point Windows at is:

```
Z:\windows-driver\lampyris-sysvad\TabletAudioSample\x64\SignedPackage\ComponentizedAudioSample.inf
```

**Ignore `ComponentizedApoSample.inf` and
`ComponentizedAudioSampleExtension.inf`.** Only `ComponentizedAudioSample.inf`
is needed for the mic. This is stated in `silence_debug_progress.md`.

### 6.2 Install or update the driver — the confirmed procedure

This is the procedure documented in `testing_instructions.md` section 4. Use it
whenever a **Lampyris Virtual Microphone** already exists in Device Manager,
which is the normal case (including every rebuild).

1. Open **Device Manager**.
2. Find the existing **Lampyris Virtual Microphone**.
3. Right-click → **Update driver**.
4. Select **Browse my computer for drivers**.
5. Select **Let me pick from a list of available drivers on my computer**.
6. Click **Have Disk...** and navigate to the `ComponentizedAudioSample.inf`
   inside `SignedPackage\` (full path above).
7. Click **Install this driver software anyway** when prompted.

**8. REBOOT THE VM.**

The reboot is not optional. `testing_instructions.md` section 5 calls it
**the critical test** — it is what proves the driver initialises correctly at
boot, not just when hot-swapped. Any step that installs or updates the driver
ends with a reboot.

Verify after the reboot: in Device Manager, expand **Audio inputs and outputs**.
You should see **Lampyris Virtual Microphone** with no warning triangle. The
probe log shows the full endpoint name as
`External Microphone Headphone (Lampyris Virtual Microphone)`.

### 6.3 Fallback: no Lampyris device exists yet

Use this ONLY if step 2 above fails because there is no Lampyris device in
Device Manager at all — for example on a brand-new VM, or after the full
uninstall in section 6.6.

This is a **root-enumerated software device**. There is no physical hardware for
Windows to detect, so it will never appear on its own, and right-clicking the
INF → Install only copies it into the driver store without creating the device.

1. Open **Device Manager**.
2. Menu **Action** → **Add legacy hardware**.
3. Next → choose **Install the hardware that I manually select from a list
   (Advanced)**.
4. Choose **Show All Devices** → Next.
5. Click **Have Disk...**.
6. Browse to
   `Z:\windows-driver\lampyris-sysvad\TabletAudioSample\x64\SignedPackage\ComponentizedAudioSample.inf`
7. Pick the Lampyris device from the list → Next → Next → Finish.
8. **Reboot the VM.**

**(this Add-legacy-hardware sequence is NOT documented in the repo — it is
inferred from the driver being root-enumerated. Confirm on first run. The
repo-confirmed procedure is the Update driver flow in 6.2, which needs an
existing device.)**

### 6.4 The `pnputil` route

Elevated command prompt, working directory the SignedPackage folder:

```
cd /d Z:\windows-driver\lampyris-sysvad\TabletAudioSample\x64\SignedPackage
pnputil /add-driver ComponentizedAudioSample.inf /install
```

Expected output shape:
```
Microsoft PnP Utility

Adding driver package:  ComponentizedAudioSample.inf
Driver package added successfully.
Published Name:         oemNN.inf
```

**Write down the `oemNN.inf` name.** You need it to fully uninstall later.
**Then reboot the VM.**

`pnputil /add-driver ... /install` is documented in the repo
(`debug_capture_instructions.md`). Whether it alone creates the root-enumerated
device is **(not verified in-repo — confirm on first run)**. If the mic does not
appear after `pnputil`, use 6.2 (device exists) or 6.3 (device does not exist).

### 6.5 Reinstall from scratch after a rebuild

The quickest correct path after a rebuild is simply 6.2 (Update driver) plus the
reboot. Use the heavier path below only when you want to be certain no old
binary survives.

1. Device Manager → find **Lampyris Virtual Microphone** (under **Audio inputs
   and outputs**, and/or **Sound, video and game controllers**).
2. Right-click → **Uninstall device**.
3. **Tick "Delete the driver software for this device"** (also worded "Attempt to
   remove the driver for this device"). This is required — without it Windows
   reuses the old binary.
4. Install the freshly signed package again (6.3, since the device is now gone).
5. **Reboot the VM.**

### 6.6 FULL uninstall — forces a fresh endpoint property store

Use this when you want Windows to build the endpoint's registry property store
from scratch. This matters because of the `OEMFormat` behaviour: the INF's
`PKEY_AudioEngine_OEMFormat` value only **seeds** the cached `DeviceFormat` the
FIRST time the endpoint's property store is created. If the store already exists,
the INF cannot repair it. A full uninstall is the durable version of the
registry fix.

All commands elevated.

**Step 1 — list installed media devices:**

```
pnputil /enum-devices /class Media
```

Find the Lampyris entry and note its **Instance ID**.

**Step 2 — remove the device:**

```
pnputil /remove-device "<Instance ID from step 1>"
```

**Step 3 — list driver packages and find yours:**

```
pnputil /enum-drivers
```

Look for the entry whose **Original Name** is `componentizedaudiosample.inf`.
Note its **Published Name** — an `oemNN.inf`.

**Step 4 — delete the driver package from the store:**

```
pnputil /delete-driver oemNN.inf /uninstall /force
```

Replace `oemNN.inf` with the Published Name from step 3.

**Step 5 — remove the leftover endpoint registry key (optional but thorough):**

```powershell
cd Z:\windows-driver
.\reset_mic_endpoint.ps1 -Purge
```

This deletes the endpoint subtree (after taking a `.reg` backup), so nothing
stale survives.

**Step 6 — reboot**, then install the fresh package. The device no longer
exists at this point, so use the 6.3 fallback (Add legacy hardware), then
**reboot again**.

**Step 7 — prove the INF fix alone works.** Run the probe (section 3b) and the
end-to-end test (section 9) **without** running `reset_mic_endpoint.ps1` at all.
If it works with no manual registry step, the INF fix is proven durable. This is
step (e) of the runbook in `silence_debug_progress.md`.

**(the exact `pnputil /enum-devices`, `/remove-device`, `/enum-drivers`, and
`/delete-driver` invocations above are standard Windows commands but are NOT
documented in this repo — only `/add-driver ... /install` is. Confirm the exact
flags on first run with `pnputil /?`.)**

---

## 7. Compiling and running the PC client (Lampyris)

### 7.1 Build

Working directory: `Z:\pc-client`. Admin not required. Any command prompt with
`cargo` on the PATH.

```
cd /d Z:\pc-client
cargo build --release
```

First build downloads and compiles all dependencies and takes a while. Later
builds are fast. **(build time not verified in-repo)**

Target triple: `x86_64-pc-windows-msvc`. `pc-client/.cargo/config.toml` adds
`-C target-feature=+crt-static`, so the exe statically links the C runtime and
needs no redistributable.

### 7.2 Where the exe lands

```
Z:\pc-client\target\release\lampyris.exe
```

(The crate name is `lampyris`, from `pc-client/Cargo.toml`.)

### 7.3 Run it

```
cd /d Z:\pc-client
cargo run --release -- --port 47999 --debug
```

or directly:

```
Z:\pc-client\target\release\lampyris.exe --port 47999 --debug
```

CLI flags (from `pc-client/src/main.rs`), only two exist:

| Flag | Short | Meaning | Default |
|---|---|---|---|
| `--port <u16>` | `-p` | TCP port to connect to | `47999` |
| `--debug` | `-d` | Raise log level from INFO to DEBUG | off |

A Slint window opens. It shows the local IP and port.

### 7.4 The log file — this is where the real information is

**Nothing useful is printed to the console.** All `tracing` output goes to a
file, with colours off:

```
%LOCALAPPDATA%\lampyris\lampyris.log
```

Typically `C:\Users\<you>\AppData\Local\lampyris\lampyris.log`.

Note: the log file is **created fresh on every start** (`File::create`), so it
only ever contains the current run.

Tail it live in a second PowerShell window:

```powershell
Get-Content -Path "$env:LOCALAPPDATA\lampyris\lampyris.log" -Wait -Tail 50
```

Leave that window open while you test. Expect an early line `Starting Lampyris
Client`.

### 7.5 USB mode and adb

When USB mode is active the client shells out to `adb forward` to tunnel the port
over the cable (`pc-client/src/app_state.rs`). It first removes any existing
forward for that port, then adds a new one, and removes it again on shutdown.
`adb` must be on your `PATH`.

Check the phone is visible:

```
adb devices
```

Expected: your device serial followed by `device`. If it says `unauthorized`,
accept the USB debugging prompt on the phone.

---

## 8. Compiling the WASAPI probe

The probe is a plain user-mode program. The driver does not need rebuilding to
use it.

**Which command prompt:** **x64 Native Tools Command Prompt for VS 2022**. `cl`
only exists there. A normal `cmd` or PowerShell will say `'cl' is not
recognized`.

Working directory: `Z:\windows-driver\tools`. No admin needed.

```
cd /d Z:\windows-driver\tools
cl /EHsc /W3 wasapi_probe.cpp ole32.lib
wasapi_probe.exe
```

That command line is taken verbatim from `debug_capture_instructions.md`.

### What the output means, field by field

**Header:**
```
Active capture endpoints: 2
```
How many active microphones Windows sees.

**Per endpoint block:**

| Line | Meaning |
|---|---|
| `[0] Microphone (High Definition Audio Device)` | Index and friendly name. Any endpoint without "Lampyris" is your **control** — it should be healthy, and proves the VM audio engine works. |
| `id: {0.0.1.00000000}.{guid}` | The endpoint id string. The `{guid}` part is the registry key name under `...\MMDevices\Audio\Capture\`. |
| `DeviceFormat raw (48 bytes): 41 00 ...` | Raw bytes of the cached `PKEY_AudioEngine_DeviceFormat`. |
| `DeviceFormat: channels=..., rate=..., bits=..., blockAlign=..., avgBytesPerSec=..., formatTag=0x...., mask=0x... (STEREO), validBits=...` | Decoded cached format. **This is the value that causes the bug when stale.** Wanted: `channels=2, rate=48000, bits=16, mask=0x3 (STEREO)`. |
| `DeviceFormat: <not set>` | The cached value is missing entirely. Also a bug state. |
| `OEMFormat: ...` | Same decode for `PKEY_AudioEngine_OEMFormat`. This one only *seeds* DeviceFormat when the endpoint store is first created. |
| `Activate(IAudioClient) -> 0x00000000  S_OK` | COM object created. Almost always succeeds. |
| `GetMixFormat -> 0x00000000  S_OK` | Windows read the cached format successfully. |
| `GetMixFormat -> 0x88890008  AUDCLNT_E_UNSUPPORTED_FORMAT` | **The failure signature of this bug.** Windows read the cached format and rejected it. |
| `fmt: 2 ch, 48000 Hz, 16 bit, tag 0xFFFE` | What GetMixFormat returned. `0xFFFE` = WAVE_FORMAT_EXTENSIBLE. |
| `Initialize(SHARED, mix fmt) -> 0x00000000  S_OK` then `(open OK — endpoint healthy)` | A normal shared-mode open worked. This is what a working mic looks like. |

**Deep probe section (Lampyris only)** — uses an explicit 2ch/48000/16 format so
it never depends on `GetMixFormat`:

| Attempt | Meaning if it succeeds |
|---|---|
| `IsFormatSupported(SHARED, dev fmt)` | The shared-mode engine accepts the format. |
| `IsFormatSupported(EXCL, dev fmt)` | The driver's own pin accepts the format. |
| `SHARED, explicit dev fmt, no event` → `>>> SHARED OPEN SUCCEEDED <<<` | Normal apps (browser, Voice Recorder) can open the mic. |
| `SHARED + EVENTCALLBACK` → `>>> EVENT-DRIVEN OPEN SUCCEEDED <<<` | Event-driven (pull) clients can open it too. |
| `EXCLUSIVE` → `>>> EXCLUSIVE OPEN SUCCEEDED — driver-side pin creation WORKS <<<` | The driver side is fine; any remaining problem is in the shared-mode engine pipe. |

**Exit codes:** `0` normal, `1` COM/enumeration failed, `2` no endpoint with
"Lampyris" in the name.

---

## 9. End-to-end test

This is the acceptance bar. Nothing counts as fixed until this passes.

The sequence below is the one recorded in `testing_instructions.md` section 5.

### 9.0 Reboot first if you just installed or updated the driver

`testing_instructions.md` calls the reboot **the critical test** — it is what
proves the driver initialises correctly from a cold boot, not just from a
hot-swap. If you have installed, updated, or reinstalled the driver since the
last restart, reboot the VM now before going any further.

### 9.1 Start the phone side (Sonus)

**The phone is the SERVER. The PC is the CLIENT.** This is deliberate — it copies
how professional hardware behaves.

1. Open the **Sonus** app on the Android phone.
2. Grant the microphone permission if asked.
3. The app starts a foreground service and listens on TCP port **47999** over
   TLS.
4. Note the IP address shown in the app.

Both devices must be able to reach each other:
- **Wi-Fi mode:** phone and PC on the same network. The PC must be able to reach
  the phone's IP on port 47999.
- **USB mode:** cable plugged in, `adb devices` shows the phone as `device`. The
  PC client sets up `adb forward` itself.

### 9.2 Run the PC client in the VM

`testing_instructions.md` runs it with the `--debug` flag. Do the same.

Terminal 1 — working directory `Z:\pc-client`. No admin needed.

```
cd /d Z:\pc-client
cargo run --release -- --debug
```

Or run the built exe directly, from wherever you copied it:

```
lampyris.exe --debug
```

Terminal 2 — tail the log (nothing useful prints to the console):

```powershell
Get-Content -Path "$env:LOCALAPPDATA\lampyris\lampyris.log" -Wait -Tail 50
```

In the Lampyris window, connect to the phone. Watch the log window for the TLS
handshake and pairing lines. On first pairing you enter the PIN shown in the
phone app.

**Expected in the log:** connection established, no repeated retry lines. The
client retries at most 5 times before giving up
(`const MAX_RETRIES: u32 = 5` in `app_state.rs`).

### 9.3 Speak into the phone and watch the green meter

This is the confirmed check from `testing_instructions.md` section 5:

1. Speak into your phone.
2. Open **Windows Sound Settings** in the VM and go to the **Recording** tab.
   (Quick route: press Win+R, type `mmsys.cpl`, press Enter, then click
   **Recording**.)
3. Look at the green volume meter next to the **Lampyris Microphone**.

**Working = the green meter lights up and bounces with your voice.**

### 9.4 Optional extra checks

- **Voice Recorder**: select the Lampyris mic, record a few seconds, play it
  back. You should hear your voice.
- A browser mic test page should show a live level and must NOT throw
  `NotReadableError`.
- Voice Recorder must NOT say "no microphone".

If you also have DebugView running, you should see `NewStream ENTER`,
`AllocBuffer`, `SetState ... -> 3` (3 = RUN), and repeating drain prints
(`ReadAudioData`, `TimerNotifyRT`, `GetPosition`).


## 10. Troubleshooting table

| Symptom | Likely cause | Exact fix |
|---|---|---|
| Driver will not install; "third-party INF does not contain digital signature information" or the device shows a yellow warning triangle | Test signing is off | Elevated: `bcdedit /set testsigning on` then **reboot**. Confirm "Test Mode" appears bottom-right. Verify with `bcdedit /enum {current}` → `testsigning  Yes`. |
| Driver install prompts about an untrusted publisher, or fails to load | Certificate not in both stores | Elevated, in `Z:\windows-driver`: `.\trust_certs.ps1`. It adds `lampyris-mic.cer` to **root** AND **TrustedPublisher**. Both are required. |
| Device fails to start with `0xC00000BB` (`STATUS_NOT_SUPPORTED`) | **Stale binary from a single-project build.** Not a real bug. | Rebuild the WHOLE solution: `cd /d Z:\windows-driver\lampyris-sysvad` then `msbuild sysvad.sln /t:Clean /p:Configuration=Release /p:Platform=x64` then `msbuild sysvad.sln /p:Configuration=Release /p:Platform=x64`. Confirm `EndpointsCommon.lib`'s timestamp updated. Then re-sign, reinstall (section 6.2), and **reboot the VM**. Do not chase this error any further — it is a known false lead. |
| `GetMixFormat -> 0x88890008 AUDCLNT_E_UNSUPPORTED_FORMAT` still after running the reset script | The registry write did not take, or audio services did not restart | 1. Confirm the script ran **elevated** (it exits with code 1 and a red message otherwise). 2. Confirm it printed a good `--- AFTER ---` block with `channels=2, rate=48000, bits=16, mask=0x3`. 3. Confirm it printed `Audiosrv status: Running`. 4. Re-run it, then reboot the VM, then re-run `wasapi_probe.exe`. |
| Reset script says `ERROR: no Lampyris capture endpoint found.` | Device not installed, or disabled | Device Manager → **Audio inputs and outputs** → enable the Lampyris mic, or install it (section 6). |
| Device appears in Device Manager and starts fine, but audio is silent, meter never moves | Client not connected / ring buffer empty, OR the deeper consumption bug | 1. Confirm the phone app is running and the PC client says connected — tail `%LOCALAPPDATA%\lampyris\lampyris.log`. 2. Confirm DebugView shows `IOCTL_LAMPYRIS_PUSH_AUDIO` prints (data arriving). 3. If `NewStream` DID fire but there is still no sound, this is a different, narrower bug in the ring-buffer / `ReadAudioData` path in `minwavertstream.cpp` — report it separately. |
| DebugView shows nothing at all | Not running as admin, or kernel capture off | Close it. Right-click `Dbgview.exe` → **Run as administrator**. Then **Capture** menu → tick **Capture Kernel**, **Enable Verbose Kernel Output**, **Capture Events**. |
| DebugView shows start-path lines but never `NewStream` | You captured at the wrong moment, OR the engine is still aborting the open | Start-path prints fire at device start (Disable/Enable in Device Manager). Open-path prints fire at mic open ("Listen to this device"). Have DebugView running BEFORE the trigger. If timing is right and `NewStream` still never fires, save the full probe output and treat it as a new investigation. |
| `'cl' is not recognized as an internal or external command` | Wrong command prompt | Close it. Open **x64 Native Tools Command Prompt for VS 2022** from the Start menu. Confirm with `cl` (expect a version banner). |
| `'msbuild' is not recognized` | Wrong command prompt | Same fix. Open **x64 Native Tools Command Prompt for VS 2022**. Confirm with `msbuild -version`. |
| `Inf2Cat not found at: ...` or `SignTool not found at: ...` | `sign_driver.ps1` hardcodes WDK `10.0.22621.0` | Run `dir "C:\Program Files (x86)\Windows Kits\10\bin"` and edit `$WdkBinRoot` in `sign_driver.ps1` to a version you have. |
| `SignTool failed to sign ...` | Certificate thumbprint mismatch | Run `.\find_cert.ps1`, then edit `$CertThumb` in `sign_driver.ps1` to the printed thumbprint. |
| `Driver release directory not found at: ...x64\Release` | You built Debug, or did not build | Rebuild with `/p:Configuration=Release /p:Platform=x64`. `sign_driver.ps1` only looks in `x64\Release`. |
| APO projects show red errors at the end of the build | **Expected.** The four APO subprojects do not build and are not needed. | Ignore them. Verify success by checking `lampyris-mic.sys` exists with a fresh timestamp in `TabletAudioSample\x64\Release\`. |
| PowerShell refuses to run a `.ps1` ("running scripts is disabled on this system") | Execution policy | Run it as: `powershell -ExecutionPolicy Bypass -File .\<script>.ps1` |
| `trust_certs.ps1` or `export_cert.ps1` cannot find the file | They contain hardcoded absolute paths | `trust_certs.ps1` now derives the path from its own folder, so it works from any drive. `export_cert.ps1` still hardcodes `D:\Border-Tech\...` — edit `$outPath` before using it. |
| Probe says `No capture endpoint with 'Lampyris' in the name.` (exit code 2) | Device missing or disabled | Device Manager → enable or install (section 6). |
| The CONTROL mic (`[0]`) also fails in the probe | The VM's audio engine itself is broken | Fix VM audio first. The driver is not the problem. This is what the probe's own interpretation key says. |

---

## 11. Rollback

### 11.1 Undo the registry change

Every run of `reset_mic_endpoint.ps1` writes a `.reg` backup into
`Z:\windows-driver\` before touching anything. The filename shape is:

```
mic_endpoint_backup_<guid-without-braces>_<yyyyMMdd-HHmmss>.reg
```

List them, newest last:

```powershell
cd Z:\windows-driver
Get-ChildItem mic_endpoint_backup_*.reg | Sort-Object LastWriteTime
```

Restore the one you want (elevated command prompt or PowerShell):

```
reg import "Z:\windows-driver\mic_endpoint_backup_<guid>_<timestamp>.reg"
```

The script itself prints this exact restore command at the end of each endpoint
block:

```
  Backup: Z:\windows-driver\mic_endpoint_backup_..._....reg   (restore with: reg import "...")
```

After importing, restart the audio stack so Windows re-reads it (elevated
PowerShell):

```powershell
Restart-Service -Name AudioEndpointBuilder -Force
Get-Service -Name Audiosrv
```

Expect `Status: Running`. All audio cuts out for a few seconds.

### 11.2 Roll back the driver

**Option A — Device Manager roll back.** Device Manager → Lampyris Virtual
Microphone → right-click → **Properties** → **Driver** tab → **Roll Back
Driver**. Only available if a previous version was installed.
**(not verified in-repo — confirm on first run)**

**Option B — full removal, then reinstall a known-good package.** Follow section
6.5 to fully remove the device and delete the driver package, then install an
older `SignedPackage` folder you kept.

**Option C — turn test signing back off.** This blocks the test-signed driver
from loading, which effectively disables it. Elevated:

```
bcdedit /set testsigning off
```

Then reboot. The "Test Mode" watermark disappears. Turn it back on with
`bcdedit /set testsigning on` plus a reboot when you want to resume.

### 11.3 Roll back the PC client

Nothing to uninstall — it is a single exe. Delete or rebuild:

```
cd /d Z:\pc-client
cargo clean
cargo build --release
```

---

## Quick command reference

| What | Folder | Elevated? | Command |
|---|---|---|---|
| Fix the cached format | `Z:\windows-driver` | **yes** | `.\reset_mic_endpoint.ps1` |
| Wipe the endpoint store | `Z:\windows-driver` | **yes** | `.\reset_mic_endpoint.ps1 -Purge` |
| Run the probe | `Z:\windows-driver\tools` | no | `wasapi_probe.exe` |
| Build the probe | `Z:\windows-driver\tools` | no (x64 Native Tools prompt) | `cl /EHsc /W3 wasapi_probe.cpp ole32.lib` |
| Build the driver | `Z:\windows-driver\lampyris-sysvad` | no (x64 Native Tools prompt) | `msbuild sysvad.sln /p:Configuration=Release /p:Platform=x64` |
| Sign the driver | `Z:\windows-driver` | yes (safest) | `powershell -ExecutionPolicy Bypass -File sign_driver.ps1` |
| Trust the cert | `Z:\windows-driver` | **yes** | `.\trust_certs.ps1` |
| Install/update the driver | Device Manager | **yes** | Update driver → Have Disk → `SignedPackage\ComponentizedAudioSample.inf`, then **reboot** |
| Find the cert thumbprint | `Z:\windows-driver` | no | `.\find_cert.ps1` |
| Turn on test signing | any | **yes** | `bcdedit /set testsigning on` then reboot |
| Build the PC client | `Z:\pc-client` | no | `cargo build --release` |
| Run the PC client | `Z:\pc-client` | no | `cargo run --release -- --port 47999 --debug` |
| Tail the client log | any | no | `Get-Content "$env:LOCALAPPDATA\lampyris\lampyris.log" -Wait -Tail 50` |

---

## Source files behind this guide

If something here disagrees with reality, these are the files to check:

- `windows-driver/silence_debug_progress.md` — root cause and the a–e runbook
- `windows-driver/debug_capture_instructions.md` — probe, build, sign, DebugView
- `windows-driver/reset_mic_endpoint.ps1` — the registry repair script
- `windows-driver/sign_driver.ps1` — WDK path, cert thumbprint, staging folder
- `windows-driver/trust_certs.ps1`, `find_cert.ps1`, `export_cert.ps1`
- `windows-driver/tools/wasapi_probe.cpp` — probe output format
- `windows-driver/lampyris-sysvad/sysvad.sln` — project list
- `windows-driver/lampyris-sysvad/TabletAudioSample/TabletAudioSample.vcxproj` — `TargetName lampyris-mic`, the `EndpointsCommon.lib` raw link input
- `pc-client/Cargo.toml`, `pc-client/.cargo/config.toml`, `pc-client/src/main.rs`
- `debug_logs/wasapi_probe.txt` — the real "before" probe output
- `process/features/windows-driver/_GUIDE.md` — current status summary
