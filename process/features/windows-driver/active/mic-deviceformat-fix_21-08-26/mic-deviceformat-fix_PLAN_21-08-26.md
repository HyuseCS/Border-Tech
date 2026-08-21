---
name: plan:mic-deviceformat-fix
description: Fix stale cached PKEY_AudioEngine_DeviceFormat causing Lampyris virtual mic silence; write INF fix + registry repair script + self-diagnosing probe
date: 21-08-26
feature: windows-driver
---

# Mic DeviceFormat Fix — Plan

**Date**: 21-08-26
**Status**: CODE DONE (21-08-26) — awaiting user verification on the Windows VM (runbook steps a-e)
**Complexity**: SIMPLE-to-moderate defect fix
**Feature**: windows-driver

## Overview

This plan fixes the Lampyris virtual microphone silence bug by writing the missing
`PKEY_AudioEngine_DeviceFormat` registry value (root cause: stale cached mix format on a
capture endpoint), both for future clean installs (INF fix) and the currently-broken
install (repair script), plus a self-diagnosing probe extension and doc updates. See
process/context/all-context.md and process/context/tests/all-tests.md for repo-wide context.

## Phase Skip Record

- **SPEC skipped** — this is a defect fix with a single unambiguous acceptance criterion
  (mic produces audible, meter-moving audio on the Windows VM). No product-discovery
  requirements doc is needed.
- **INNOVATE skipped** — the corrective action is mechanical once the root cause is known
  (write the missing registry value two ways: INF for future installs, PowerShell script
  for the existing broken install). There is no competing design to weigh.
- RESEARCH is complete and is treated as given (see the root-cause block below). This plan
  translates that research directly into an implementation checklist.

## Root Cause (established, not re-investigated)

`IAudioClient::GetMixFormat` on a **capture** endpoint reads the cached registry value
`PKEY_AudioEngine_DeviceFormat` (`{F19F064D-082C-4E27-BC73-6882A1BB8E4C},0`) under
`HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Capture\{endpoint-GUID}\Properties`.
It does **not** query the driver at all. Commit `634691d` (21 Jun) collapsed the MicIn pin's
wave-format table down to a single stereo 2ch/48000/16-bit format and left the endpoint's
`Properties` registry key stale from an earlier mono-only pin table. Commit `a86f997` correctly
added `PKEY_AudioEngine_OEMFormat` (`{E4870E26-3CC5-4CD2-BA46-CA0A9A70ED04},3`) to the INF with a
byte-correct stereo blob, but `OEMFormat` only seeds `DeviceFormat` the **first time** an
endpoint's property store is created — this endpoint's store already existed (created 30 Jun per
`EventViewerLogs.md`), so the seed never took effect. The INF never writes `DeviceFormat` directly.
Net effect: the audio engine believes the mix format is stale/unsupported, aborts the capture
open before calling `NewStream`, and the meter never moves.

Design decision (already made, not re-litigated here): **keep the stereo 2ch/48000/16 contract.**
Do not revert to mono — `pc-client/src/audio/windows.rs:132-136` already upmixes mono to stereo,
and the pin tables / jack descriptor / INF blob are internally consistent at stereo. Reverting
would widen blast radius across `pc-client`, `android-client`, and the driver for no benefit.

Ruled out (do not re-chase): event-driven/notification mismatch, malformed
WAVEFORMATEXTENSIBLE math, wrong dwChannelMask, KS data-range inconsistency, wrong pin index,
PROPOSEDATAFORMAT2 mode handling, the IOCTL/ring-buffer data path, `MicInMiniports` wave-interface
args, driver start path, ffmpeg/DirectShow probes. `windows-driver/driver.cpp` +
`lampyris-mic.vcxproj` are dead code (build a separate, unused `\Device\LampyrisMic`) — noted for
cleanup only, not touched by this plan.

## Touchpoints

| File | Change |
|---|---|
| `windows-driver/lampyris-sysvad/TabletAudioSample/ComponentizedAudioSample.inx` | Add `PKEY_AudioEngine_DeviceFormat` string to `[Strings]`; add matching `HKR,EP\0,...` REG_BINARY line to `[SYSVAD.I.TopologyMicIn.AddReg]` right after the existing OEMFormat line; update the explanatory comment above the OEMFormat block to record the real root cause |
| `windows-driver/reset_mic_endpoint.ps1` (new) | Registry repair/reset script — read-decode-write-decode BEFORE/AFTER, `.reg` backup, `-Purge` mode |
| `windows-driver/tools/wasapi_probe.cpp` | Extend the per-endpoint loop to print raw + decoded `PKEY_AudioEngine_DeviceFormat` and `PKEY_AudioEngine_OEMFormat` (via the already-open `pProps` property store) before the `GetMixFormat` call |
| `windows-driver/silence_debug_progress.md` | Rewrite: record confirmed root cause, move event-driven theory to ruled-out, new verification procedure |
| `windows-driver/debug_capture_instructions.md` | Update capture instructions to reference the new probe output and the reset script |
| `process/features/windows-driver/_GUIDE.md` | Add a new "## Current Status" section (file has no such heading today — nearest existing sections are "## Scope" and "## Debug Notes (read these first)") |

No changes to `pc-client/`, `android-client/`, `windows-driver/driver.cpp`, `windows-driver/ioctl.h`,
or any pin/format table — the stereo contract is unchanged everywhere.

## Public Contracts

- The IOCTL contract (`ioctl.h`) is unchanged.
- The wire protocol (`'MC'` framing) is unchanged.
- The advertised wave format (2ch/48000/16-bit) is unchanged — this plan makes the **cached
  registry mix format** match what was already advertised, it does not change the contract itself.
- No new public surface is introduced.

## Blast Radius

Small, single-package (`windows-driver/`), no cross-component contract change. One item is
**high-risk**:

- **`reset_mic_endpoint.ps1` writes to `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Capture\*\Properties`** on the user's Windows VM — this is live OS audio-engine configuration, not source code. Mitigations required by this plan: (1) positive endpoint identification only (name match + two known fallback GUIDs — never touch an unmatched endpoint), (2) mandatory `.reg` export backup of the exact keys before any write, (3) idempotent writes (safe to re-run), (4) print BEFORE/AFTER evidence so the user can see exactly what changed, (5) require elevation and refuse to run without it.
- The INF change (`.inx`) only takes effect for a brand-new property store (clean install) — it cannot itself fix the currently-broken install; the PowerShell script is required for that.
- `wasapi_probe.cpp` and the `.md` doc updates are zero-risk (read-only diagnostic, documentation).

## Acceptance Criteria

1. INF (`ComponentizedAudioSample.inx`) writes `PKEY_AudioEngine_DeviceFormat` with the correct
   stereo blob in `[SYSVAD.I.TopologyMicIn.AddReg]`, alongside the existing OEMFormat entry.
2. `reset_mic_endpoint.ps1` exists, is idempotent, requires elevation, backs up via `.reg` export
   before any write, and prints BEFORE/AFTER decoded format values.
3. `wasapi_probe.cpp` prints decoded `DeviceFormat`/`OEMFormat` for every enumerated capture
   endpoint before calling `GetMixFormat`.
4. `silence_debug_progress.md`, `debug_capture_instructions.md`, and the feature `_GUIDE.md` are
   updated to reflect the confirmed root cause.
5. User-executed runbook step (d) — end-to-end audible audio with meter movement on the Windows
   VM — is the ultimate acceptance bar; steps (a)-(c) are diagnostic checkpoints toward it.

## Phase Completion Rules

This is a SIMPLE-to-moderate single-plan defect fix — no phase-program split. The plan is
considered CODE DONE when Implementation Checklist items 1-4 are complete and validated by the
agent (compiles / diffs correctly). It is considered VERIFIED only after the user completes the
Verification Procedure runbook (item 5) on the Windows VM and confirms audible, meter-moving
audio — do not mark VERIFIED on agent-side completion alone.

## Implementation Checklist

1. **INF fix** — `windows-driver/lampyris-sysvad/TabletAudioSample/ComponentizedAudioSample.inx`:
   a. In `[Strings]` (near line 551, alongside `PKEY_AudioEngine_OEMFormat`), add:
      `PKEY_AudioEngine_DeviceFormat = "{F19F064D-082C-4E27-BC73-6882A1BB8E4C},0"`
   b. In `[SYSVAD.I.TopologyMicIn.AddReg]` (line ~175-185), immediately after the existing
      `HKR,EP\0,%PKEY_AudioEngine_OEMFormat%,0x00000001,<blob>` line, add:
      `HKR,EP\0,%PKEY_AudioEngine_DeviceFormat%,0x00000001,<same 48-byte blob>`
      (byte-for-byte identical blob to the OEMFormat line — this is the same
      2ch/48000/16-bit WAVEFORMATEXTENSIBLE serialization).
   c. Keep the existing OEMFormat line untouched. Update the comment block above (currently
      "Default endpoint format (PKEY_AudioEngine_OEMFormat): serialized VT_BLOB PROPVARIANT")
      to state: DeviceFormat is the value the engine actually reads on every open; OEMFormat only
      seeds it on first-ever property-store creation, which is why the endpoint stayed stale after
      the OEMFormat fix landed.
   d. Use flag `0x00000001` (REG_BINARY, clobber/overwrite — NOT `0x00010001` NOCLOBBER) so a
      reinstall always overwrites any stale cached value.

2. **Registry repair script** — new file `windows-driver/reset_mic_endpoint.ps1`:
   a. `#Requires -RunAsAdministrator` (or explicit elevation check that exits with a clear
      message if not elevated).
   b. Enumerate `HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Capture\*`.
      For each subkey, read the friendly name from property `{a45c254e-df1c-4efd-8020-67d146a850e0},2`
      under `...\Properties`. Select an endpoint if its name matches `*Lampyris*` (case-insensitive)
      OR its GUID matches `{50ff5a10-7084-4bad-85d0-9f8a616b5524}` or
      `{b1dd805e-09b6-4673-8efd-2f072d9bf0cf}` (documented fallback identifiers from
      `EventViewerLogs.md`). If zero matches: print a clear error and exit non-zero without
      writing anything.
   c. For each matched endpoint: before any write, export the endpoint's `Properties` key to
      `windows-driver\mic_endpoint_backup_{endpoint-guid}_{yyyyMMdd-HHmmss}.reg` via `reg export`.
   d. Read and print the BEFORE value (hex bytes) of `{F19F064D-082C-4E27-BC73-6882A1BB8E4C},0`
      (DeviceFormat) and `{E4870E26-3CC5-4CD2-BA46-CA0A9A70ED04},3` (OEMFormat). Decode each as a
      WAVEFORMATEXTENSIBLE (skip the leading VT_BLOB/length header, parse `wFormatTag`,
      `nChannels`, `nSamplesPerSec`, `wBitsPerSample`, `dwChannelMask`) and print in plain text
      (e.g. "channels=1, rate=44100, bits=16, mask=0x4 (MONO)"). If the value is absent, print
      "DeviceFormat: <not set>" explicitly — do not error.
   e. Write both properties as `REG_BINARY` with the correct 48-byte stereo blob (hardcode the
      same hex bytes as the INF: `41 00 00 00 28 00 00 00 FE FF 02 00 80 BB 00 00 00 EE 02 00 04
      00 10 00 16 00 10 00 03 00 00 00 01 00 00 00 00 00 10 00 80 00 00 AA 00 38 9B 71`).
   f. Read back and print the AFTER values, decoded the same way, so the user can visually confirm
      the fix landed (expect "channels=2, rate=48000, bits=16, mask=0x3 (STEREO)").
   g. Restart the audio stack: stop `AudioEndpointBuilder` and dependent services (which includes
      `Audiosrv`) in dependency order, then start them again (`Restart-Service AudioEndpointBuilder
      -Force` should cascade-restart `Audiosrv` since it depends on it; verify with
      `Get-Service Audiosrv` after and explicitly `Start-Service Audiosrv` if it did not come back).
      Print a console warning immediately before this step: restarting these services briefly
      interrupts ANY other active audio (music, calls, other mics) on the machine — expected and
      harmless, but the user should know before it happens.
   h. Support a `-Purge` switch: instead of writing corrected values, delete the entire matched
      endpoint `Properties`/registry subtree for Lampyris capture endpoints (still after the `.reg`
      backup step) and print instructions to disable/re-enable the Lampyris device in Device
      Manager so Windows recreates the endpoint fresh from the INF's OEMFormat seed.
   i. Never touch any endpoint that did not positively match step (b). No wildcard fallback.

3. **Self-diagnosing probe** — `windows-driver/tools/wasapi_probe.cpp`:
   a. In the existing per-endpoint enumeration loop (around line 120-140, where
      `pDev->OpenPropertyStore` is already called and `PKEY_Device_FriendlyName` is already read),
      add: print the endpoint ID string (`IMMDevice::GetId`), then read and print raw hex +
      decoded WAVEFORMATEXTENSIBLE fields for `PKEY_AudioEngine_DeviceFormat`
      (`{F19F064D-082C-4E27-BC73-6882A1BB8E4C},0`) and `PKEY_AudioEngine_OEMFormat`
      (`{E4870E26-3CC5-4CD2-BA46-CA0A9A70ED04},3`) from the SAME already-open `pProps` store,
      BEFORE the existing `GetMixFormat` call. Handle `VT_EMPTY`/missing value explicitly
      ("DeviceFormat: <not set>").
   b. Reuse the existing `PROPVARIANT`/`PropVariantClear` pattern already in the file (see lines
      128-131) — do not introduce a new property-reading idiom.
   c. This applies to every enumerated capture endpoint (the control sweep loop), not just
      Lampyris — so a future run on any machine is self-explanatory without extra tooling.
   d. **Insertion point precision (validate-agent finding):** the new PKEY reads MUST be inserted
      between the existing `PropVariantClear(&v);` and `pProps->Release();` lines (the FriendlyName
      block, current ~line 128-131) — NOT merely "somewhere before `GetMixFormat`". `pProps` is
      released well before `GetMixFormat` is called later in the loop; reading from it after
      `Release()` uses a freed property store. **PKEY constant availability:** `mmdeviceapi.h` and
      `functiondiscoverykeys_devpkey.h` are already included and already resolve
      `PKEY_Device_FriendlyName`; if `PKEY_AudioEngine_DeviceFormat` and/or
      `PKEY_AudioEngine_OEMFormat` fail to resolve at compile/link time, add manual
      `DEFINE_PROPERTYKEY` declarations for them in `wasapi_probe.cpp` using the GUID/PID pairs
      given above, immediately above their first use.

4. **Documentation**:
   a. Rewrite `windows-driver/silence_debug_progress.md`: replace "Leading suspect" (event-driven
      mode) with a "## Confirmed root cause" section describing the stale `DeviceFormat` registry
      value; move the event-driven theory into a "ruled out" bullet; replace "NEXT ACTION" with the
      new verification procedure (Item 5 below); keep the existing ruled-out list and the
      build/capture gotchas sections as-is (still true and still load-bearing).
   b. Update `windows-driver/debug_capture_instructions.md` to reference: (i) running
      `reset_mic_endpoint.ps1` before any capture attempt, (ii) the extended `wasapi_probe.exe`
      output now showing DeviceFormat/OEMFormat decode.
   c. `process/features/windows-driver/_GUIDE.md` has no existing "## Current Status" heading —
      add a new one near the top (after "## Scope", before "## Key Source Files") stating the root
      cause is identified and the fix is implemented-but-user-unverified (pending Windows VM run).

## Verification Procedure (numbered runbook — user-executed on the Windows VM)

**Ordering is deliberate: steps (a)-(d) require NO driver rebuild and must run first.** They
directly test the theory and the registry-level fix. Only after they pass does step (e) rebuild
the driver to make the fix durable for future clean installs. Do not reorder — rebuilding first
wastes a rebuild cycle before the theory is confirmed.

a. **Run the reset script**: `.\reset_mic_endpoint.ps1` (elevated). Capture the printed BEFORE
   decode.
   - Expected (confirms root cause): DeviceFormat is absent, or decodes to mono / non-48000Hz /
     non-16-bit — i.e. stale relative to the current stereo pin table.
   - Branch if BEFORE already shows correct 2ch/48000/16 stereo: root cause is refuted for this
     machine; stop here and re-open the investigation (do not proceed to b-e; return to RESEARCH).

b. **Re-run the probe**: `wasapi_probe.exe`.
   - Expected: `GetMixFormat -> S_OK` returning `2 ch, 48000 Hz, 16 bit`, and shared-mode
     `Initialize` (both with and without EVENTCALLBACK) succeeding (`S_OK`) for the Lampyris
     endpoint.
   - Branch if `GetMixFormat` still fails or returns a different format: the registry write in (a)
     did not take effect (check for restart/permission issues) — re-run (a) with more diagnostic
     output before proceeding.

c. **Watch DebugView** during a real open (browser "Listen to this device" or Voice Recorder).
   - Expected: `NewStream`, `AllocateAudioBuffer`, and `SetState` LAMPYRIS-DEBUG lines appear in
     the log for the first time (per the existing instrumentation in `minwavert.cpp` /
     `minwavertstream.cpp`).
   - Branch if they still do not appear: the engine is still aborting before the miniport —
     capture the exact HRESULT chain from `wasapi_probe.exe` and treat as a new investigation.

d. **End-to-end audio test**: start `pc-client` and the Android app streaming, then test in a
   browser mic-permission page or Windows Voice Recorder.
   - Expected: audible playback, VU meter moves.
   - Branch if silent but c passed: revisit the ring-buffer/`ReadAudioData` consumption path
     (`minwavertstream.cpp:1541-1542`) — this would be a new, narrower bug, not this one.

e. **Only after a-d all pass**: make the fix durable for future clean installs.
   - Clean full-solution rebuild: `msbuild sysvad.sln /p:Configuration=Release /p:Platform=x64`
     (WHOLE SOLUTION — never a single project; a single-project build previously relinked a stale
     `EndpointsCommon.lib` and produced a false `0xC00000BB` failure).
   - Re-sign: `sign_driver.ps1`.
   - Uninstall the current Lampyris mic device WITH driver deletion (Device Manager → Uninstall
     device → check "Delete the driver software for this device").
   - Reinstall using the freshly built+signed `SignedPackage\ComponentizedAudioSample.inf`.
   - Confirm: repeat probe (b) and end-to-end test (d) on this CLEAN install with **no manual
     registry step** — this proves the INF fix (Item 1) alone is now sufficient for new installs.

## Verification Evidence

| Gate / Scenario | Strategy | Proves SPEC criterion |
|---|---|---|
| Runbook step (a) — reset script BEFORE/AFTER decode | Agent-Probe (user-executed, visually judged) | Confirms/refutes root cause: stale cached DeviceFormat |
| Runbook step (b) — `wasapi_probe.exe` GetMixFormat + Initialize | Agent-Probe (user-executed) | Registry fix makes the audio engine accept the format |
| Runbook step (c) — DebugView NewStream/AllocateAudioBuffer/SetState | Agent-Probe (user-executed) | Miniport capture stream actually opens (the original silence bug) |
| Runbook step (d) — end-to-end audible audio + meter movement | Agent-Probe (user-executed) | Overall acceptance criterion: mic produces audible input on Windows |
| Runbook step (e) — clean rebuild + reinstall with no manual step | Agent-Probe (user-executed) | INF fix (Item 1) alone durably fixes fresh installs, not just this session |
| `wasapi_probe.cpp` extended decode output compiles and runs on every enumerated endpoint | Fully-Automated (compiles via `cl /EHsc /W3 wasapi_probe.cpp ole32.lib`) | Diagnostic tooling itself is correct (necessary but not sufficient for the fix) |
| INF/script blob byte-identity check | Fully-Automated: `grep -oE "AudioEngine_(DeviceFormat|OEMFormat)%.*" windows-driver/lampyris-sysvad/TabletAudioSample/ComponentizedAudioSample.inx \| grep -oE '[0-9A-Fa-f]{2}' \| tr -d '\n'` compared against the equivalent extraction from `reset_mic_endpoint.ps1`'s hardcoded byte array — both must produce the identical 96-hex-char string (48 bytes) | Catches drift between the INF blob and the script's hardcoded blob (the two copies this plan requires to stay byte-identical) |
| INF has both `PKEY_AudioEngine_DeviceFormat` `[Strings]` entry and matching `AddReg` line | Fully-Automated: `grep -c "PKEY_AudioEngine_DeviceFormat" windows-driver/lampyris-sysvad/TabletAudioSample/ComponentizedAudioSample.inx` exits 0 and prints `2` | Confirms Implementation Checklist items 1a and 1b both landed |
| `reset_mic_endpoint.ps1` syntax parses cleanly | Agent-Probe (no `pwsh`/`powershell` in this repo's Linux dev environment — mark syntax review as agent/human visual check; if a pwsh toolchain becomes available: `pwsh -NoProfile -Command "[void][System.Management.Automation.PSParser]::Tokenize((Get-Content -Raw windows-driver/reset_mic_endpoint.ps1), [ref]$null)"` upgrades this to Hybrid) | Script has no gross syntax errors before the user ever runs it elevated against live registry keys |

There is no fully-automated or hybrid tier available for the actual fix — see Known Gap below.
This matches the existing project convention (`process/context/tests/all-tests.md`): the Windows
driver has no automated tests; verification is build-clean + manual run.

## Known Gap

The orchestrator cannot execute or verify any of this: the driver and the registry it patches run
on a Windows VM that only the user can drive. Every verification step above is user-executed, not
agent-executed. This is recorded as a known-gap, not a blocker — EXECUTE will write the code
changes (INF, script, probe extension, docs) and the compile-only checks it can run
(`cl /EHsc` on the probe if a Windows toolchain is reachable from the execution environment,
otherwise a syntax/diff review), and the user completes the runbook above afterward.

## Test Infra Improvement Notes

(none identified yet)

## Rollback Plan

- **INF change**: revert the two added lines (`[Strings]` entry + `AddReg` line) via `git revert`
  or manual diff revert. No registry side effects until a driver reinstall happens.
- **Registry script**: every write is preceded by a `.reg` export backup
  (`windows-driver\mic_endpoint_backup_{guid}_{timestamp}.reg`). Rollback = double-click the
  backup `.reg` file (or `reg import <backup>.reg`) to restore the exact prior state, then restart
  `AudioEndpointBuilder`/`Audiosrv`.
- **`-Purge` mode**: no direct rollback beyond the `.reg` backup (also taken before purge) —
  re-import the backup to restore the deleted keys.
- **Probe/docs changes**: pure revert via git, zero runtime risk.

## Resume and Execution Handoff

1. **Selected plan file path**: `process/features/windows-driver/active/mic-deviceformat-fix_21-08-26/mic-deviceformat-fix_PLAN_21-08-26.md`
2. **Last completed phase or step**: PLAN — this file just written; SPEC and INNOVATE deliberately
   skipped (see Phase Skip Record above).
3. **Validate-contract status**: pending — VALIDATE has not yet run.
4. **Supporting context files loaded**: `process/context/all-context.md`,
   `process/context/planning/all-planning.md`, `process/context/tests/all-tests.md`,
   `process/features/windows-driver/_GUIDE.md`, `windows-driver/silence_debug_progress.md`.
5. **Next step for a fresh agent picking up mid-execution**: run VALIDATE against this plan
   (`ENTER VALIDATE MODE`), then EXECUTE the Implementation Checklist in order (items 1-4 are all
   agent-executable code/doc changes; item 5, the verification runbook, is entirely user-executed
   on the Windows VM and is out of agent scope — see Known Gap).


Next step: say **ENTER VALIDATE MODE** to validate this plan, then **ENTER EXECUTE MODE** to implement.

## Validate Contract

Status: CONDITIONAL
Date: 21-08-26
date: 2026-08-21
generated-by: outer-pvl

Parallel strategy: sequential
Rationale: signal score 0/7 by the 7-signal table (single package `windows-driver/`, no multi-package scope, no 3+ competing directions, not a phase program, blast radius is 6 small files with one high-risk script) — this is a single-agent VALIDATE pass. All 4 Layer 1 dimensions and 6 Layer 2 sections were still run (not skipped), just by one agent reading serially rather than fanned out, because the plan is small enough that fan-out overhead would exceed the review cost.

Test gates (C3 5-column table):

| criterion id | behavior | strategy | proving test | gap-resolution |
|---|---|---|---|---|
| AC1-strings | INF `[Strings]` gains `PKEY_AudioEngine_DeviceFormat` entry and `[SYSVAD.I.TopologyMicIn.AddReg]` gains the matching `HKR,EP\0,...` line | Fully-Automated | `grep -c "PKEY_AudioEngine_DeviceFormat" windows-driver/lampyris-sysvad/TabletAudioSample/ComponentizedAudioSample.inx` must print `2` | A |
| AC1-blob | INF's new DeviceFormat blob is byte-identical to the existing OEMFormat blob (both must decode to 2ch/48000Hz/192000Bps/blockAlign4/16bit/cbSize22/validBits16/mask0x3/PCM) | Fully-Automated | `grep -oE "AudioEngine_(DeviceFormat\|OEMFormat)%.*" windows-driver/lampyris-sysvad/TabletAudioSample/ComponentizedAudioSample.inx \| grep -oE '[0-9A-Fa-f]{2}' \| tr -d '\n'` — the two 96-hex-char extractions must be identical. Independently re-verified by this VALIDATE pass via manual field decode against `MicInPinSupportedDeviceFormats[0]` in `micinwavtable.h` — confirmed byte-correct (see Dimension findings). | A |
| AC1-flag | INF uses clobber flag `0x00000001` (not NOCLOBBER `0x00010001`) on the new DeviceFormat line | Fully-Automated | `grep "PKEY_AudioEngine_DeviceFormat%,0x00000001," windows-driver/lampyris-sysvad/TabletAudioSample/ComponentizedAudioSample.inx` exits 0 | A |
| AC2-safety | `reset_mic_endpoint.ps1` implements all 5 mandated safety controls: elevation check, positive endpoint ID before write, `.reg` backup before first write, hard refusal on zero-match, `-Purge` opt-in only | Agent-Probe | Execute-agent self-check against the 5-point list (see Execute-Agent Instructions E1) before reporting DONE; `pwsh` is not installed in this repo's Linux dev environment so no automated tokenizer run is possible here | B |
| AC2-syntax | `reset_mic_endpoint.ps1` parses without gross syntax errors | Known-Gap (this environment) / Hybrid (if pwsh available) | `pwsh -NoProfile -Command "[void][System.Management.Automation.PSParser]::Tokenize((Get-Content -Raw windows-driver/reset_mic_endpoint.ps1), [ref]$null)"` — not runnable here (`pwsh`/`powershell` both absent from PATH in this dev environment); falls back to agent visual review | D |
| AC3-compile | `wasapi_probe.cpp` extended decode block compiles cleanly | Known-Gap (this environment) / Fully-Automated (on Windows) | `cl /EHsc /W3 wasapi_probe.cpp ole32.lib` — not runnable here (no MSVC toolchain on this Linux host); user runs this as part of runbook step (b) | D |
| AC3-order | New PKEY reads execute while `pProps` is still open (inserted before `pProps->Release()`, not merely "before `GetMixFormat`") | Fully-Automated | `grep -B2 "pProps->Release" windows-driver/tools/wasapi_probe.cpp` — the new `PKEY_AudioEngine_DeviceFormat`/`PKEY_AudioEngine_OEMFormat` read block must appear in those preceding lines | B |
| AC4-docs | `silence_debug_progress.md`, `debug_capture_instructions.md`, and `_GUIDE.md` reflect the confirmed root cause | Fully-Automated | `grep -q "Confirmed root cause" windows-driver/silence_debug_progress.md && grep -q "Current Status" process/features/windows-driver/_GUIDE.md` both exit 0 | A |
| AC5-e2e | Mic produces audible, meter-moving audio on the Windows VM (ultimate acceptance bar) | Known-Gap | User-executed Verification Procedure runbook steps (a)-(e) in the plan — no automated or hybrid gate exists or can exist for this component (kernel driver + live Windows audio engine state on a VM only the user can drive; matches `process/context/tests/all-tests.md` project-wide known gap: "No tests for the kernel driver... verified only by running the driver on a real or virtual Windows host") | D |

gap-resolution legend:
- A — proven now (gate passes in this cycle, mechanically re-checkable post-EXECUTE)
- B — fixed in this plan (gate added by this plan's checklist / this validate-fix cycle's edits)
- C — deferred to a named later phase/plan
- D — backlog test-building stub (named residual; keep-active; continue)

C-4 reconciliation: `strategy:` column carries only Fully-Automated / Hybrid / Agent-Probe as proving strategies; the two `Known-Gap` rows above (AC3-compile, AC5-e2e; AC2-syntax also Known-Gap in this environment) are named residuals via gap-resolution D, never proving strategies themselves.

Failing stubs (Fully-Automated rows only):

```
test("should have exactly 2 PKEY_AudioEngine_DeviceFormat occurrences in the INF (Strings + AddReg)", () => {
  throw new Error("NOT IMPLEMENTED — TDD stub: AC1-strings")
})
test("should have byte-identical DeviceFormat and OEMFormat blobs in the INF", () => {
  throw new Error("NOT IMPLEMENTED — TDD stub: AC1-blob")
})
test("should use clobber flag 0x00000001 on the new DeviceFormat AddReg line, not NOCLOBBER", () => {
  throw new Error("NOT IMPLEMENTED — TDD stub: AC1-flag")
})
test("should insert the new PKEY reads before pProps->Release(), not merely before GetMixFormat", () => {
  throw new Error("NOT IMPLEMENTED — TDD stub: AC3-order")
})
test("should update silence_debug_progress.md and _GUIDE.md to reflect the confirmed root cause", () => {
  throw new Error("NOT IMPLEMENTED — TDD stub: AC4-docs")
})
```

Dimension findings:
- Infra fit: PASS — single-package change confined to `windows-driver/`; no container, port, or service-lifecycle surface touched; IOCTL contract (`ioctl.h`) and wire protocol (`'MC'` framing) confirmed unchanged.
- Test coverage: CONCERN — no Fully-Automated or Hybrid gate exists (or can exist within this plan) for the actual audio fix (AC5-e2e); this is a named, justified residual documented in the plan's own "Known Gap" section and consistent with the project-wide gap already catalogued in `process/context/tests/all-tests.md` ("no automated tests for the kernel driver"). Per the net-gate vacuous-green rule this alone forces the net gate to CONDITIONAL rather than PASS, even though every code-artifact behavior (AC1, AC3-order, AC4) now has a real Fully-Automated gate added in this VALIDATE pass.
- Breaking changes: PASS — no public contract, schema, or cross-component API change; `pc-client/`, `android-client/`, `driver.cpp`, `lampyris-mic.vcxproj` confirmed untouched by grep of the plan text.
- Security surface: PASS — `reset_mic_endpoint.ps1`'s specified design (Implementation Checklist item 2) includes all 5 mandated controls: elevation check (2a), positive endpoint identification with hard refusal on zero-match (2b, 2i), `.reg` backup export before any write (2c), BEFORE/AFTER decoded evidence printed (2d, 2f), and `-Purge` as an opt-in switch never the default (2h). None of these were missing or weakened — no FAIL. Enforced again at EXECUTE via Execute-Agent Instruction E1 below since the script does not exist as code yet.
- Section 1 — INF fix: PASS — `[Strings]` (line 551 region) and `[SYSVAD.I.TopologyMicIn.AddReg]` (line 175-185 region) both confirmed to exist exactly where the plan says; existing OEMFormat line at line 185 already uses clobber flag `0x00000001` (plan correctly avoids NOCLOBBER); the 48-byte blob was independently field-decoded by this VALIDATE pass and confirmed byte-correct against `MicInPinSupportedDeviceFormats[0]` in `micinwavtable.h` (2ch / 48000Hz / 192000 avgBytesPerSec / blockAlign 4 / 16 bits / cbSize 22 / validBits 16 / mask 0x3 STEREO / SubFormat GUID `00000001-0000-0010-8000-00AA00389B71` = `KSDATAFORMAT_SUBTYPE_PCM`).
- Section 2 — reset_mic_endpoint.ps1: PASS — all 5 safety controls specified (see Security surface above); minor gap found and fixed in this pass (P4 below — restarting `AudioEndpointBuilder`/`Audiosrv` interrupts other active audio on the machine; the plan now requires a console warning before that step).
- Section 3 — wasapi_probe.cpp probe extension: CONCERN, fixed in this pass — two gaps found: (1) the plan's original wording ("before the existing `GetMixFormat` call") was ambiguous about insertion point; the actual code releases `pProps` well before `GetMixFormat` runs, so an execute-agent following only the original wording could insert the new reads after `pProps->Release()` and dereference a freed property store. Fixed via P3 below (new checklist item 3.d) with an exact anchor. (2) The plan never addressed whether `PKEY_AudioEngine_DeviceFormat`/`PKEY_AudioEngine_OEMFormat` resolve from the already-included `mmdeviceapi.h`/`functiondiscoverykeys_devpkey.h` — this cannot be confirmed without a Windows/MSVC toolchain (untested runtime/toolchain behavior, not mechanically checkable from this Linux host). Fixed via P3 with a documented `DEFINE_PROPERTYKEY` fallback instruction, so execute-agent has a defined path either way instead of being blocked on an unverifiable assumption.
- Section 4 — Documentation: CONCERN, fixed in this pass — `process/features/windows-driver/_GUIDE.md` has no existing "## Current Status" heading (verified sections present: Scope, Key Source Files, Debug Notes, Related Context, Gotchas); the plan referenced updating a section that does not exist. Fixed via P1/P2 below (add a new section instead of updating a nonexistent one).
- Section 5 — Runbook ordering and falsifiability: PASS — steps (a)-(e) run in the mandated no-rebuild-first order with an explicit rationale sentence; step (a) has an explicit refutation branch ("root cause is refuted for this machine; stop here... return to RESEARCH"); the rebuild step names the exact whole-solution command and the stale-`EndpointsCommon.lib` gotcha from `silence_debug_progress.md`.
- Section 6 — Blast radius exclusions: PASS — confirmed via grep of the plan text that `pc-client/`, `android-client/`, `windows-driver/driver.cpp`, `windows-driver/ioctl.h`, and `android-client/app/build/` are named as explicitly untouched, and no Implementation Checklist item references them.

Open gaps:
- AC5-e2e (end-to-end audible audio): Known-Gap, D — see Test gates table above. Cannot be resolved by any amount of further plan-fix cycling; it is a structural limitation of this component (kernel driver on a VM only the user can drive), already documented project-wide in `process/context/tests/all-tests.md`. Accepted as the residual reason this gate is CONDITIONAL, not PASS.
- AC2-syntax and AC3-compile (PowerShell parse / MSVC compile): Known-Gap in this Linux dev environment only — both become live Fully-Automated/Hybrid gates the moment the plan is executed on the Windows VM; no action needed now.

What this coverage does NOT prove:
- The INF/blob/flag/probe-order/doc-heading Fully-Automated gates (AC1-strings, AC1-blob, AC1-flag, AC3-order, AC4-docs) prove the CODE ARTIFACTS are byte-correct and structurally present. They do NOT prove the Windows audio engine actually re-reads the corrected registry value, that the capture stream opens (`NewStream`/`AllocateAudioBuffer`/`SetState`), or that audio is audible — those are exactly AC5-e2e and the runbook steps (a)-(d), which remain user-executed.
- The AC2-safety Agent-Probe self-check proves the script's SPECIFICATION was followed at write-time. It does not prove the script behaves correctly against a live, differently-configured Windows registry until the user actually runs it (runbook step a) — that first live run IS the test, by design (idempotent, backed up, and printing BEFORE/AFTER for the user to visually confirm).
- Section 5/6 PASS verdicts prove the plan's runbook text and stated exclusions are internally consistent; they do not prove the user will execute the runbook correctly or that no other file gets touched during EXECUTE — that is confirmed post-EXECUTE by re-grepping the blast radius.

Gate: CONDITIONAL (0 FAILs; 1 irreducible CONCERN — no automated/hybrid proof exists for the actual audio fix, which is inherent to this component and already documented as a project-wide known gap; the other 2 CONCERNs found in this VALIDATE pass — probe insertion-point ambiguity and a nonexistent doc-section target — were both fixed directly in this pass, see Proposed Plan Updates below)
Accepted by: session (autonomous, /goal execution) — per explicit user grant of full autonomy for this task ("run the PVL loop automatically... do not pause for approval"). One plan-fix cycle was run and applied in this same VALIDATE pass (see Proposed Plan Updates P1-P5); the remaining CONCERN (AC5-e2e) is not fixable by further plan cycling — it is accepted as a named, justified residual, not a silent pass.

### Proposed Plan Updates (applied in this pass)

| # | What changed | Where in plan | Why |
|---|---|---|---|
| P1 | Touchpoints row for `_GUIDE.md` corrected from "Update 'Current Status' section" to "Add a new '## Current Status' section (none exists today)" | Touchpoints table | `_GUIDE.md` has no such heading; the original wording would have sent execute-agent looking for a section that doesn't exist |
| P2 | Implementation Checklist item 4.c reworded to say "add a new '## Current Status' heading near the top" instead of "update" the (nonexistent) heading | Implementation Checklist item 4c | Same reason as P1 |
| P3 | Added new Implementation Checklist item 3.d: exact insertion-point anchor (before `pProps->Release()`) + `DEFINE_PROPERTYKEY` fallback instruction for the two PKEY constants | Implementation Checklist item 3 | Closes the pProps-use-after-release risk and the unverifiable PKEY-constant-availability assumption found in Layer 2 Section 3 |
| P4 | Added a console-warning requirement to Implementation Checklist item 2.g before restarting `AudioEndpointBuilder`/`Audiosrv` | Implementation Checklist item 2g | Restarting these services interrupts all other active audio on the machine momentarily; the user should be told before it happens, not surprised by it |
| P5 | Added 3 new rows to the Verification Evidence table: INF/script blob byte-identity check (Fully-Automated), INF PKEY-string-count check (Fully-Automated), PS1 syntax parse (Agent-Probe / Hybrid-if-`pwsh`-available) | Verification Evidence table | Converts 3 previously-implicit checks into explicit, exact, re-runnable gates — reduces the CONDITIONAL surface to the one irreducible gap (AC5-e2e) |

### Execute-Agent Instructions

| # | Instruction | Trigger condition |
|---|---|---|
| E1 | Before reporting DONE on `reset_mic_endpoint.ps1`, re-verify all 5 safety controls are present in the actual written code (not just intended): elevation check exits early if not elevated; the endpoint-match logic runs and can return zero matches BEFORE any registry-write call; `.reg export` is the first write-adjacent action and happens before the `REG_BINARY` write; the zero-match path never reaches a write; `-Purge` is a switch parameter that is absent/off by default. If any control is missing or weaker than specified here, fix it before reporting DONE — this is a hard gate, not a note. | `reset_mic_endpoint.ps1` creation, before DONE |
| E2 | After extending `wasapi_probe.cpp`, run `grep -B2 "pProps->Release" windows-driver/tools/wasapi_probe.cpp` and confirm the new PKEY read block appears in the 2 lines immediately before `pProps->Release()`. If it does not, move it — do not leave it after the Release call. | `wasapi_probe.cpp` edit, before DONE |
| E3 | After the INF edit, run the INF/script blob-identity grep (AC1-blob in the Test gates table) once `reset_mic_endpoint.ps1` exists. If the two blobs diverge, fix immediately in the same EXECUTE pass — do not defer to a follow-up. | After both INF and `reset_mic_endpoint.ps1` are written |


## Autonomous Goal Block

SESSION GOAL: Fix the Lampyris virtual mic silence bug — write the missing `PKEY_AudioEngine_DeviceFormat` registry value (INF for future installs + PowerShell repair script for the current install), extend the WASAPI probe to self-diagnose it, and update the debug docs.
Charter + umbrella plan: N/A — single plan (no phase program)
Autonomy: full autonomy granted by user for this task. EXECUTE and UPDATE PROCESS may proceed without pausing for approval; CONDITIONAL gaps are auto-accepted per orchestration.md §Autonomy Mode (irreversible/outward-facing actions still hard-stop).
Hard stop conditions / safety constraints:
- `reset_mic_endpoint.ps1` must never write to an endpoint that did not positively match by name (`*Lampyris*`) or one of the two documented fallback GUIDs — no wildcard fallback, ever.
- Every registry write must be preceded by a `.reg` export backup of the exact keys being changed.
- `-Purge` (key deletion) must be an opt-in switch, never the default action.
- The script must refuse to run without elevation.
- Do not touch `pc-client/`, `android-client/`, `windows-driver/driver.cpp`, `windows-driver/ioctl.h`, or `android-client/app/build/` — out of scope for this plan.
- Windows driver rebuilds must always be whole-solution (`msbuild sysvad.sln /p:Configuration=Release /p:Platform=x64`) — never single-project (stale `EndpointsCommon.lib` relink risk).
Next phase: EXECUTE: process/features/windows-driver/active/mic-deviceformat-fix_21-08-26/mic-deviceformat-fix_PLAN_21-08-26.md
Validate contract: inline in plan (see `## Validate Contract` above)
Execute start: Fully-automated gates — `grep -c "PKEY_AudioEngine_DeviceFormat" windows-driver/lampyris-sysvad/TabletAudioSample/ComponentizedAudioSample.inx` (expect 2), the INF/script blob-identity grep (AC1-blob), `grep -B2 "pProps->Release" windows-driver/tools/wasapi_probe.cpp` (AC3-order) | Agent-probe: reset_mic_endpoint.ps1 5-point safety self-check (E1) | Known-gap (user-executed on Windows VM): runbook steps (a)-(e) | high-risk pack: recommended, not blocking — `reset_mic_endpoint.ps1` writes live HKLM audio config; see vc-risk-evidence-pack for the optional manual-first evidence artifacts if the user wants a formal record before running it against real hardware.

## Deviations (EXECUTE, 21-08-26)

All within blast radius; no hard-stop class. Full detail in
`mic-deviceformat-fix_REPORT_21-08-26.md` §Plan Deviations.

1. `_GUIDE.md` already had a `## Current Status` heading (contrary to P1/P2) — rewritten in
   place instead of adding a duplicate. Same end state.
2. INF comment says "DeviceFormat"/"OEMFormat" without the `PKEY_AudioEngine_` prefix so the
   AC1-strings gate still prints exactly `2`.
3. Probe PKEY constants declared locally as `PKEY_Lampyris_AudioEngine_*` (per the item-3d
   `DEFINE_PROPERTYKEY` fallback) with distinct names, so they cannot clash with an SDK that
   already defines the standard names.
4. Probe gained an `else` branch printing the endpoint name when the property store cannot be
   opened — preserves pre-existing per-endpoint name output.
