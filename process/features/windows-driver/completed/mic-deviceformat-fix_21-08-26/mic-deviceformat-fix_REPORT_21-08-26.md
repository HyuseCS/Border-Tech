---
phase: mic-deviceformat-fix
date: 2026-08-21
status: COMPLETE_WITH_GAPS
feature: windows-driver
plan: process/features/windows-driver/active/mic-deviceformat-fix_21-08-26/mic-deviceformat-fix_PLAN_21-08-26.md
---

# EXECUTE Report — Mic DeviceFormat Fix

## What Was Done

| File | Change |
|---|---|
| `windows-driver/lampyris-sysvad/TabletAudioSample/ComponentizedAudioSample.inx` | Added `PKEY_AudioEngine_DeviceFormat = "{F19F064D-082C-4E27-BC73-6882A1BB8E4C},0"` to `[Strings]` (line 563) and a matching `HKR,EP\0,%PKEY_AudioEngine_DeviceFormat%,0x00000001,<48-byte blob>` line to `[SYSVAD.I.TopologyMicIn.AddReg]` (line 196), directly after the untouched OEMFormat line. Rewrote the comment block above to record the real root cause (DeviceFormat is read on every open; OEMFormat only seeds on first store creation). |
| `windows-driver/reset_mic_endpoint.ps1` (new, 277 lines) | Registry repair script: elevation gate, positive endpoint identification, `.reg` backup, BEFORE/AFTER WAVEFORMATEXTENSIBLE decode, idempotent `REG_BINARY` write, audio-stack restart with a pre-warning, opt-in `-Purge` mode. |
| `windows-driver/tools/wasapi_probe.cpp` | Added local `DEFINE_PROPERTYKEY` declarations + `PrintEngineFormatProperty()`; the Pass-1 endpoint loop now prints the endpoint id and raw + decoded `DeviceFormat`/`OEMFormat` from the still-open `pProps` store, immediately before `pProps->Release()`. Handles `<not set>` explicitly. |
| `windows-driver/silence_debug_progress.md` | Replaced `## Leading suspect` with `## Confirmed root cause`; moved the event-driven theory into the ruled-out list; replaced `## NEXT ACTION` with the 5-step (a)-(e) verification runbook. Ruled-out list and build/capture gotchas kept as-is. |
| `windows-driver/debug_capture_instructions.md` | "Where we are" now states the confirmed root cause; new `## STEP 0` (run `reset_mic_endpoint.ps1` before any capture attempt); STEP 1 renamed "(after STEP 0)" and documents the new probe decode output. |
| `process/features/windows-driver/_GUIDE.md` | `## Current Status` rewritten: symptom, confirmed root cause, the two-half fix, and the user-executed verification still outstanding. |

## What Was Skipped or Deferred

Nothing in checklist items 1-4. Item 5 (the verification runbook) is user-executed on the Windows VM and out of agent scope by design.

## Test Gate Outcomes

| Gate | Command | Result |
|---|---|---|
| AC1-strings | `grep -c "PKEY_AudioEngine_DeviceFormat" <inx>` | **PASS** — printed `2` |
| AC1-flag | `grep "PKEY_AudioEngine_DeviceFormat%,0x00000001," <inx>` | **PASS** — clobber flag, not NOCLOBBER |
| AC1-blob | hex extraction from INF vs `$StereoBlob` in the `.ps1` | **PASS** — `4100000028000000FEFF020080BB000000EE02000400100016001000030000000100000000001000800000AA00389B71` (96 hex chars / 48 bytes) identical across INF DeviceFormat, INF OEMFormat, and the script |
| AC3-order | `grep -B2 "pProps->Release" wasapi_probe.cpp` | **PASS** — `PrintEngineFormatProperty(...)` calls appear in the two lines before `pProps->Release()` |
| AC4-docs | `grep -q "Confirmed root cause" ... && grep -q "Current Status" ...` | **PASS** |
| AC2-safety (E1, Agent-Probe) | manual 5-point re-verify against written code | **PASS** — elevation (L1 + L68-71, exit 1); positive-match only (L163/L165, no wildcard branch); zero-match refusal `exit 2` at L181-186 precedes every write; `reg export` at L208 precedes `Remove-Item` (L225) and `New-ItemProperty` (L241-242); `-Purge` is `[switch]` at L32, off by default |
| AC2-syntax | `pwsh -NoProfile ...Tokenize(...)` | **NOT RUN** — `pwsh` and `powershell` are both absent from PATH on this Linux host (`which` returned not-found for both). Agent visual review only. Known gap D, unchanged. |
| AC3-compile | `cl /EHsc /W3 wasapi_probe.cpp ole32.lib` | **NOT RUN** — no MSVC toolchain on this host. User runs it as runbook step (b). Known gap D, unchanged. |
| AC5-e2e | user runbook (a)-(e) | **NOT RUN** — structural known gap D; Windows VM only. |

Blast-radius re-check: `git status` confirms no change to `driver.cpp`, `ioctl.h`, `lampyris-mic.vcxproj/.sln`, `pc-client/`, or `android-client/` source. The dirty `android-client/app/build/**` entries are pre-existing committed build output present at session start; untouched by this phase.

## Plan Deviations

1. **`_GUIDE.md` already had a `## Current Status` heading** (line 41). The plan (P1/P2, from the VALIDATE pass) said none existed and instructed to add one. Adding a second would have produced a duplicate heading, so the existing section was rewritten in place with the required content. Within blast radius; same end state; AC4-docs gate unaffected.
2. **INF comment wording avoids the literal string `PKEY_AudioEngine_DeviceFormat`** (uses "DeviceFormat"). Necessary so the AC1-strings gate keeps printing exactly `2` as the contract specifies. Semantics of the comment are unchanged.
3. **Probe PKEY constants declared locally as `PKEY_Lampyris_AudioEngine_DeviceFormat` / `..._OEMFormat`** rather than relying on SDK-provided names. Checklist item 3d authorised a `DEFINE_PROPERTYKEY` fallback; distinct names were used so the declarations cannot clash with an SDK that already defines the standard names. This makes the compile deterministic instead of SDK-version-dependent.
4. **Probe also prints a name line when the property store fails to open** (`else` branch), preserving the pre-existing behavior that every endpoint prints its index and name. Not in the plan; required to avoid a regression introduced by moving the `wprintf` inside the property-store block.

No hard-stop-class deviations. No schema, auth, API, billing, or container-lifecycle surface touched.

## Test Infra Gaps Found

- No PowerShell toolchain (`pwsh`/`powershell`) on this Linux dev host — PowerShell scripts in `windows-driver/` cannot be syntax-checked locally. Installing `powershell-bin` would upgrade AC2-syntax from Known-Gap to Hybrid.
- No MSVC toolchain — `wasapi_probe.cpp` cannot be compile-checked locally.
- Both are pre-existing, environment-level, and consistent with `process/context/tests/all-tests.md` ("no automated tests for the kernel driver").

## Closeout Packet

- **Selected plan**: `process/features/windows-driver/active/mic-deviceformat-fix_21-08-26/mic-deviceformat-fix_PLAN_21-08-26.md`
- **Finished**: Implementation Checklist items 1-4 in full.
- **Verified**: all 5 Fully-Automated gates green; AC2-safety agent-probe green.
- **Unverified**: AC2-syntax, AC3-compile, AC5-e2e — all user-executed on the Windows VM.
- **Remaining**: user runs the runbook in `silence_debug_progress.md` ("NEXT ACTION"), starting with step (a) and the BEFORE decode.
- **Best next state**: **Keep in active/testing.** Code is complete but the plan's own Phase Completion Rules say VERIFIED requires the user's Windows VM run (step d, audible meter-moving audio). Do not archive yet.

## Forward Preview

**Test Infra Found**: none automatable added; the two environment gaps above persist.

**Blast Radius Changes**: none beyond the plan's 6 files (+1 new `.ps1`).

**Commands to Stay Green**:
```
grep -c "PKEY_AudioEngine_DeviceFormat" windows-driver/lampyris-sysvad/TabletAudioSample/ComponentizedAudioSample.inx   # expect 2
grep -q "PKEY_AudioEngine_DeviceFormat%,0x00000001," windows-driver/lampyris-sysvad/TabletAudioSample/ComponentizedAudioSample.inx
grep -B2 "pProps->Release" windows-driver/tools/wasapi_probe.cpp | grep -q PKEY_Lampyris_AudioEngine
grep -q "Confirmed root cause" windows-driver/silence_debug_progress.md
```
Plus the INF-vs-PS1 blob-identity extraction (AC1-blob) — the two copies of the 48-byte blob must never drift.

**Dependency Changes**: none.

**Follow-up plan stubs created**: none.

**CONTEXT_PARTIAL items**: none.

---

## UPDATE PROCESS Closeout Packet (2026-08-21)

1. **Selected plan path**: `process/features/windows-driver/active/mic-deviceformat-fix_21-08-26/mic-deviceformat-fix_PLAN_21-08-26.md`

2. **Closeout classification**: **Keep in active/testing.** Implementation is complete and all Fully-Automated + Agent-Probe gates are green, but AC5-e2e (the acceptance criterion — audible, meter-moving audio on the Windows VM) is unverified. The plan itself declares VERIFIED status requires the user's Windows VM run. Not archived.

3. **What was finished**: INF fix (writes `PKEY_AudioEngine_DeviceFormat` for future clean installs), `reset_mic_endpoint.ps1` repair script (fixes the currently-broken install), probe extension (`wasapi_probe.cpp` now decodes and prints `DeviceFormat`/`OEMFormat` per endpoint), and doc updates (`silence_debug_progress.md`, `debug_capture_instructions.md`, feature `_GUIDE.md`).

4. **Verified vs unverified**: Verified — AC1-strings, AC1-blob, AC1-flag, AC3-order, AC4-docs (all Fully-Automated), AC2-safety (Agent-Probe, 5-point manual code re-verify). Unverified — AC2-syntax (no `pwsh`/`powershell` on this Linux host), AC3-compile (no MSVC toolchain), AC5-e2e (user-executed on Windows VM; no automated/hybrid gate can exist for a kernel-mode capture path).

4b. **Validate-contract compliance**: VALIDATE ran for this plan; gate history recorded in the plan file (`Gate: PASS`/prior cycles). Present.

5. **Cleanup done vs still needed**: Done this session — `process/context/all-context.md` (Current State, High-Risk Areas, Key Files, Open Questions) and `process/features/windows-driver/_GUIDE.md` (already updated by execute-agent, verified consistent) reconciled to the confirmed root cause. Still needed — nothing process-side; the only remaining work is the user's Windows VM verification run.

6. **Single best next valid state**: Keep the plan active. User runs the verification runbook (`silence_debug_progress.md` → "NEXT ACTION" steps a-e) on the Windows VM. On success, re-enter UPDATE PROCESS MODE to archive the plan; on failure, return to PLAN/RESEARCH with the new evidence.

7. **Commit-checkpoint recommendation**: Not applicable this session — user explicitly declined a commit. When ready: implementation changes (INF, `.ps1`, probe, docs) are a natural single execution commit; this closeout packet + context updates are process-only and can follow in the same or a separate commit per user preference.

8. **Regression status**: N/A — single-plan defect fix, not a phase program; no prior verified surfaces to regress against. Blast-radius re-check in the EXECUTE report confirms no drift into `driver.cpp`, `ioctl.h`, `pc-client/`, or `android-client/` source.

9. **SPEC achievement**: No SPEC for this plan — SPEC was explicitly skipped (single unambiguous acceptance criterion, defect fix; see plan's "Phase Skip Record"). N/A.

**Drift score: LOW** (1 signal: >10 files touched region-wise but concentrated in one feature area; no `.claude`/`.codex`/agent-harness files changed; no README/AGENTS/CLAUDE/protocol files changed; context updates captured but session was a single defect-fix pass, not 3+ memory-worthy architectural decisions beyond the one root-cause finding; no task-folder structural change — folder already existed; no validate-contract deviation). UPDATE PROCESS available if you want.

**Next valid state**: Keep the plan active and continue validation on the same selected plan — awaiting the user's Windows VM verification run.
