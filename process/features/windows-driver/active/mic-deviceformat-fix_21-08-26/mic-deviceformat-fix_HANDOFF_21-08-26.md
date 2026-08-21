---
name: report:mic-deviceformat-fix-handoff
description: Next-session handoff for the Lampyris mic silence fix — root cause, ruled-out list, shipped commits, and the decision tree for the user's Windows VM verification result
date: 21-08-26
metadata:
  node_type: memory
  type: report
  feature: windows-driver
  phase: mic-deviceformat-fix
---

# Mic DeviceFormat Fix — Next-Session Handoff

**Read this file first. Do not re-read the 434-line plan unless this handoff sends you there.**

## 1. Situation

The Lampyris virtual mic (Windows kernel driver) outputs silence. The root cause is
established and the code fix is complete, but it is **unverified** — nobody has run it on
the Windows VM yet. The user is returning next session with a console output line from
that verification. Your entire job is to read that line and route to the correct next
action using the decision tree in section 6. Do not restart the investigation.

## 2. Root Cause (compact)

For a **capture** endpoint, `IAudioClient::GetMixFormat` reads the cached registry value
`PKEY_AudioEngine_DeviceFormat` (`{F19F064D-082C-4E27-BC73-6882A1BB8E4C},0`) under
`HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Capture\{GUID}\Properties`.
It does **not** query the driver.

Commit `634691d` narrowed the MicIn pin to a single 2ch/48000/16-bit stereo format and
deleted every mono format. It never touched the endpoint's cached `DeviceFormat`. Commit
`a86f997` added `PKEY_AudioEngine_OEMFormat`, but `OEMFormat` only seeds `DeviceFormat` at
**first-ever property-store creation** — this endpoint's store already existed, so the seed
never took effect.

Net effect: the audio engine reads a stale/unsupported cached mix format, aborts the
capture open before calling `NewStream`, and the meter never moves.

## 3. Evidence Table

| Observation | Explanation |
|---|---|
| `GetMixFormat` fails | Engine reads the stale cached `DeviceFormat`, not the driver's pin table |
| `IsFormatSupported(EXCLUSIVE)` succeeds but `SHARED` fails | Shared mode is the path that consults the cached mix format; exclusive mode goes straight to the pin |
| Miniport returns `STATUS_SUCCESS` to every KS query | The driver is healthy — the engine never reaches it |
| `NewStream` never fires | Engine aborts the open before calling into the miniport at all |
| Adding `OEMFormat` (commit `a86f997`) changed nothing | `OEMFormat` only seeds on first store creation; this store already existed |

Evidence sources: `debug_logs/wasapi_probe.txt`, `debug_logs/WINDOWSVM.log`.

## 4. Ruled Out — Do Not Re-Chase

Five independent research passes converged on the root cause above. These theories were
tested and killed. Do not re-investigate them without new contradicting evidence.

- **Event-driven/notification-contract mismatch** — refuted empirically: `Initialize` fails
  identically with and without `AUDCLNT_STREAMFLAGS_EVENTCALLBACK`.
- **Malformed `WAVEFORMATEXTENSIBLE`** — arithmetic verified: `nBlockAlign` 4,
  `nAvgBytesPerSec` 192000, all fields internally consistent.
- **Wrong `dwChannelMask`** — it is `KSAUDIO_SPEAKER_STEREO` (`0x3`), correct.
- **KS data-range mismatch** — `MicInPinDataRangesStream` is 2ch/16-16bit/48000-48000Hz,
  consistent with the single advertised format.
- **Wrong pin index** — Pin 1 IS the host capture pin, instances 5/5, confirmed.
- **`PROPOSEDATAFORMAT2` mode handling** — honest per-mode table match, no bug.
- **The IOCTL/ring-buffer data path** — fully wired: `pc-client` → `\\.\LampyrisMic2` →
  `lampyris_core.cpp` → `minwavertstream.cpp:1541-1542`. Producer side is healthy.
- **Driver start path** — every start step returns `0x0`.
- **The earlier `0xC00000BB` (`STATUS_NOT_SUPPORTED`) failed-start** — was a **stale
  binary**: a single-project build relinking a stale `EndpointsCommon.lib`. Not a real bug.
  Always do a clean whole-solution rebuild.
- **ffmpeg/DirectShow probes** — red herring; legacy `KsProxy` mishandles WaveRT capture.
  The real clients (browser, Voice Recorder, WASAPI) use a different path.
- **Missing `SysvadWaveFilterInterfacePropertiesCapture` on `MicInMiniports`** — that
  `0, NULL` is stock SYSVAD behavior, not a defect.

## 5. What Shipped (7 commits, this session, branch `feat/windows`)

```
9eaca2e fix(windows-driver): set PKEY_AudioEngine_DeviceFormat so the mic endpoint opens
02f60cc chore(windows-driver): dump endpoint DeviceFormat and OEMFormat in wasapi_probe
116cd00 docs(windows-driver): record confirmed root cause of the mic silence bug
a629db4 process(windows-driver): add mic-deviceformat-fix plan, validate contract and closeout
6993791 docs(windows-driver): add Windows VM build, install and test guide
cc064f2 chore(android-client): untrack generated Gradle build output
1c19d9d fix(android-client): store gradlew with LF line endings
```

Nothing pushed. Working tree clean at session end.

## 6. THE DECISION TREE

The user returns with the **BEFORE** line printed by
`windows-driver\reset_mic_endpoint.ps1`. Route on it exactly as follows.

### Branch A — BEFORE shows a stale/mismatched format

Any of: `channels=1`, a non-48000 rate, a mono mask, or `<not set>`.

→ **ROOT CAUSE CONFIRMED.** Continue the runbook in
`windows-driver/silence_debug_progress.md` → `## NEXT ACTION`:

- **(b)** Re-run `wasapi_probe.exe`. Expect `GetMixFormat -> S_OK, 2 ch, 48000 Hz`.
- **(c)** Watch DebugView during a real open. Expect `NewStream`, `AllocateAudioBuffer`,
  `SetState` LAMPYRIS-DEBUG lines to appear for the first time.
- **(d)** End-to-end test: start `pc-client` + Android app streaming, test in a browser or
  Voice Recorder. Expect audible playback, VU meter moves. **This is the acceptance bar.**
- **(e)** Only after (a)-(d) all pass: whole-solution rebuild
  (`msbuild sysvad.sln /p:Configuration=Release /p:Platform=x64` — never single-project),
  re-sign, uninstall-with-driver-deletion, reinstall from the signed INF, then repeat (b)
  and (d) on the clean install with **no manual registry step** — this proves the INF fix
  alone is durable for future installs.

### Branch B — BEFORE already shows `channels=2, rate=48000, mask=0x3` AND `GetMixFormat` still fails

→ **THEORY REFUTED.** Do NOT continue the runbook. Re-enter RESEARCH. Specific next lines
of investigation, in order:

1. **Endpoint identity mismatch.** Event Viewer logs show TWO capture endpoints from this
   driver: `{50ff5a10-7084-4bad-85d0-9f8a616b5524}` and a second, "UNKNOWN"-named
   `{b1dd805e-09b6-4673-8efd-2f072d9bf0cf}`. Confirm the endpoint the reset script matched
   is the SAME endpoint `wasapi_probe.exe` opened — compare endpoint ID strings. A stale
   duplicate endpoint would explain a correct-looking value on the wrong key.
2. Whether an APO/FX property on the endpoint is rejecting the format.
3. Whether the `KSDATARANGE_ATTRIBUTES` signal-processing-mode attribute list is malformed.

### Branch C — Script errors or refuses to run

The script is designed to refuse rather than guess. Capture the exact message. Common
causes: not elevated, no endpoint matched by name (`*Lampyris*`) or either fallback GUID,
`reg export` backup failed. This is expected behavior, not a bug — fix the stated cause and
re-run.

### Branch D — `GetMixFormat` succeeds but audio is still silent

→ **NEW, NARROWER BUG. Not this one.** The registry fix worked; something else is silent.
Investigate the ring-buffer consumption path at `minwavertstream.cpp:1541-1542` and whether
`pc-client` is actually pushing data — check `%LOCALAPPDATA%\lampyris\lampyris.log`.

## 7. Known Gaps Carried Forward

Three things were never verifiable on this Linux dev host and remain genuinely open:

- `reset_mic_endpoint.ps1` has never been syntax-parsed — no `pwsh`/`powershell` installed.
- The extended `wasapi_probe.cpp` has never been compiled — no MSVC toolchain.
- No audio has ever been heard.

The script and the probe were reviewed line-by-line against real PowerShell/C++ semantics
and no defects were found — but reviewed is not run. A syntax error on first run is
possible and is **not** evidence against the diagnosis; it just means fix the syntax and
re-run.

## 8. Where Everything Lives

- **Operational runbook:** `windows-driver/WINDOWS_VM_GUIDE.md` — the single document to
  follow at the Windows VM, copy-pasteable start to finish. It **supersedes**
  `testing_instructions.md` (root) — that file is older and shorter; do not follow it.
- **Debug history:** `windows-driver/silence_debug_progress.md` — full ruled-out list,
  confirmed root cause, and the numbered verification runbook.
- **The contract:** this plan's `## Validate Contract` and `## Verification Procedure`
  sections in
  `process/features/windows-driver/active/mic-deviceformat-fix_21-08-26/mic-deviceformat-fix_PLAN_21-08-26.md`.
- **Repo context:** `process/context/all-context.md`.

## 9. First Actions for a Fresh Agent

1. Read this handoff file in full (you just did).
2. Read the plan's `## Validate Contract` section (Test gates table + Gate verdict) —
   skim only, do not re-derive it.
3. If the user has not already pasted the BEFORE line from `reset_mic_endpoint.ps1`, ask
   for it.
4. Route per the decision tree in section 6.

**Do not re-run the research.** Five independent passes already converged on the root
cause in section 2, and the ruled-out list in section 4 is evidence-backed. Re-opening that
investigation without new contradicting evidence (i.e. Branch B firing) wastes the user's
time and yours.
