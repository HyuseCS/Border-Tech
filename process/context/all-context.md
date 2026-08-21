# Project-M - All Context

Last updated: 2026-08-21

This file is the root context entrypoint for the repo.

Use it for two things:

1. quick routing to the right context pack or root file
2. broad architecture and repository understanding

Start here before loading deeper context files.

---

## What This Project Is

**Project-M** (repo name) is the **Border Tech** phone-to-PC ecosystem: a suite that turns an Android phone into a real input device for a desktop workstation. Two products ship today, backed by one kernel driver:

- **Lampyris** (`pc-client/`) — the desktop hub. Rust + Slint. It is the *client*: it dials the phone over TLS, authenticates with SRP, unwraps the PCM stream, and pushes it into the OS audio graph. It is designed as the central hub for future phone-to-PC feature apps, not just audio.
- **Sonus** (`android-client/`) — the Android app. Kotlin + Compose. It is the *server*: it captures 48kHz mono PCM and serves it over TLS on port 47999.
- **Lampyris Mic driver** (`windows-driver/`) — a kernel-mode WDM + SYSVAD virtual microphone so Windows apps see the phone as a normal mic. On Linux the equivalent job is done in user space by PipeWire.

**Who it is for:** gamers, developers, podcasters, and workstation users — originally on Arch/CachyOS — who want a lossless, low-latency, open-source replacement for proprietary phone-as-mic tools.

**Owner:** solo developer. No PR review process; conventions are set by the existing code.

### Current State (2026-08-21, branch `feat/windows`)

**The Windows virtual microphone works end to end.** Verified on a Win10 22H2 VM
on 2026-08-21: one capture endpoint named `Lampyris Virtual Microphone (Lampyris
Mic)`, opens in shared / event-driven / exclusive mode, correct pitch, no
perceptible delay, confirmed in Voice Recorder and in a live Discord call.

Four separate bugs were fixed to get there, each hiding the next:

1. **Stereo-only capture pin** (`micinwavtable.h`) — the shared-mode capture pipe
   will not open a stereo-only capture endpoint. Total silence, `NewStream` never
   called. Introduced by `634691d`.
2. **Fake stereo frames** (`pc-client/src/audio/windows.rs`) — the client wrote
   each mono sample twice as an L/R pair. Against a mono pin the driver read each
   pair as two samples, so playback ran at half speed: same words, one octave
   down.
3. **Unbounded ring buffer** (`lampyris_core.cpp`) — the client pushes from the
   moment it connects but nothing drains until an app opens the mic, so the ring
   filled to its full 2 s and the reader stayed that far behind. Fixed 1–2 s
   delay, now capped at a 100 ms watermark.
4. **MicIn's own descriptor set could not be opened by the audio engine**
   (`minipairs.h`) — it now uses MicArray's topology descriptor, wave descriptor,
   packet-size constraints and format/mode table, keeping only its own identity.
   The pin itself was provably fine throughout: `IsFormatSupported(EXCLUSIVE,
   1ch/48000/16)` returned `S_OK` the whole time.

Every capture mode is pinned to 48 kHz mono, because the driver does no
resampling and the wire protocol is fixed at mono 48 kHz s16le. MicArray's stock
table mapped SPEECH to 16 kHz and COMMUNICATIONS to 24 kHz, which communications
apps select by default.

The full ruled-out list, the two Windows facts that cost real time, and the
verification runbook are in `windows-driver/silence_debug_progress.md`. Read it
before touching the driver — several plausible-looking theories were already
tested and killed.

Build and install with `windows-driver/rebuild_install.ps1` (elevated): build →
sign → purge every stale `oem*.inf` → install → reboot prompt. It judges the
build by whether `lampyris-mic.sys` was produced.

### High-Risk Areas

All four of these are treated as hot spots. Change them deliberately.

| Area | Where | Why it is risky |
|---|---|---|
| Windows kernel driver | `windows-driver/` | Kernel mode, SYSVAD-derived, test-signing required. Hard to test, easy to bugcheck. Always build the **whole solution** — `TabletAudioSample.vcxproj` links `EndpointsCommon.lib` as a raw input with no `<ProjectReference>`, so a single-project build silently relinks a stale library and discards your changes. Always purge every stale `ComponentizedAudioSample` package from the driver store before installing; five copies once accumulated and it was unclear which the device was bound to. `rebuild_install.ps1` does both correctly. |
| Audio backends | `pc-client/src/audio/` | Two `cfg`-gated implementations behind one trait. A change on one platform silently skips compilation on the other. |
| TLS + SRP auth path | `pc-client/src/app_state.rs`, `AudioCaptureService.kt` | Security-critical: custom cert verifier, rcgen self-signed certs, SRP/SPAKE2 pairing. Constant-time comparison matters here. |
| Android capture loop | `android-client/app/src/main/java/com/projectm/mic/AudioCaptureService.kt` | Foreground service + `AudioRecord` loop. Latency and dropout regressions show up here first. |

---

## How This File Works (the `all-*.md` Convention)

Every `process/context/` directory has one `all-*.md` entrypoint that acts as an attachable quick router for that domain. This root file (`all-context.md`) is the top-level router. Context groups each have their own `all-{group}.md` entrypoint.

**The pattern:**

```
process/context/
  all-context.md                      <-- THIS FILE: root router
  planning/
    all-planning.md                   <-- group router for planning
    example-simple-prd.md             <-- deep doc within the group
    example-complex-prd.md            <-- deep doc within the group
  tests/
    all-tests.md                      <-- group router for tests
    debugging-and-pitfalls.md         <-- deep doc within the group
    e2e-tests.md                      <-- deep doc within the group
  database/
    all-database.md                   <-- group router for database
    schema-guide.md                   <-- deep doc within the group
    migration-procedures.md           <-- deep doc within the group
```

**How agents use it:**

1. Agent reads `all-context.md` first (this file)
2. Finds the relevant context group from the routing tables below
3. Reads that group's `all-{group}.md` entrypoint
4. Only then loads the specific deep doc needed

This layered routing keeps context windows small. Never load the whole `process/context/` tree.

**What each `all-{group}.md` must contain:**

- Scope (what the group covers and does NOT cover)
- Read-when rules (when an agent should load this group)
- Quick procedures or decision rules
- Source paths (list of deeper docs in the group)
- Update triggers (when to refresh this group's content)
- Routing to deeper docs within the group

---

## Quick Start

For most substantial tasks:

1. read this file first
2. choose the smallest relevant root file or context group from the tables below
3. only then load deeper files

---

## Current Root Entry Points

<!-- The two tables below (Root Entry Points + Context Groups) are GENERATED from each
     context doc's frontmatter by `discover-context.mjs --emit-routing`. Do NOT hand-edit
     between the GENERATED markers — your edits will be overwritten on the next rebuild.
     To change a row, edit the owning doc's frontmatter (description / keywords) and re-emit.
     `--check-routing` fails lint if this block drifts from the frontmatter on disk. -->

<!-- GENERATED:routing -->
| File | Read when |
|---|---|
| `process/context/all-context.md` | any substantial planning, research, review, or implementation task |
| `process/context/planning/all-planning.md` | Planning conventions and plan-shape calibration for Project-M: when to use the SIMPLE vs COMPLEX plan format, and example PRDs. |
| `process/context/tests/all-tests.md` | Verification guide for Project-M: which runner to use per component, exact commands, debugging quirks, and the known test-coverage gaps. |

## Current Context Groups

| Group | Entry point | Scope |
|---|---|---|
| `planning/` | `process/context/planning/all-planning.md` | Planning conventions and plan-shape calibration for Project-M: when to use the SIMPLE vs COMPLEX plan format, and example PRDs. |
| `tests/` | `process/context/tests/all-tests.md` | Verification guide for Project-M: which runner to use per component, exact commands, debugging quirks, and the known test-coverage gaps. |
<!-- /GENERATED:routing -->

## Task Routing Table

<!-- Routing entries below reflect the context groups that actually exist. -->
<!-- The "Load first" column always starts with all-context.md. -->
<!-- The "Then load" column points to the group entrypoint, then optionally a deep doc. -->

| If the task involves... | Load first | Then load |
|---|---|---|
| architecture or stack questions | this file | the relevant feature guide under `process/features/` |
| testing or verification | this file, `process/context/tests/all-tests.md` | the component's source dir |
| creating a new plan | this file, `process/context/planning/all-planning.md` | the relevant feature folder `active/` |
| Rust PC hub work (Lampyris) | this file | `process/features/pc-client-lampyris/_GUIDE.md` |
| Android app work (Sonus) | this file | `process/features/android-client-sonus/_GUIDE.md` |
| Windows kernel driver work | this file | `process/features/windows-driver/_GUIDE.md` |
| TLS / SRP / pairing work | this file | `pc-client/src/app_state.rs`, `android-client/app/src/main/java/com/projectm/mic/AudioCaptureService.kt` |
| wire protocol / framing work | this file | `pc-client/src/protocol.rs` (has the only unit tests) |
| context maintenance | this file | run `vc-audit-context` after edits |

## Context Group Lifecycle

Context groups are durable knowledge domains, not feature folders.

Create a group when:

- a topic has 3+ durable docs
- a single doc exceeds roughly 800 lines with separable subtopics
- multiple agents repeatedly need only one slice of a large context file
- the topic maps to a stable operational domain (tests, infra, database, auth, UI, workflows, etc.)

Do not create a group when:

- the content is a temporary report
- the content is a plan or execution artifact
- the topic is feature-specific and belongs in `process/features/...`

Move or split one group at a time. Use `all-{group}.md` entrypoints. Run the `audit-context` skill after every context organization change.

## Naming Convention

There are no `README.md` files inside `process/context/`.

Canonical entrypoints use `all-*.md`:

- root: `process/context/all-context.md`
- group: `process/context/{group}/all-{group}.md`

Each `all-{group}.md` file should act as the attachable quick router for that domain:

- tell the agent what the group covers
- give quick procedures and decision rules
- route to smaller deeper files

## Context Update Protocol

When durable project knowledge changes:

1. update the smallest relevant context file
2. update this file if routing, ownership, naming, or groups changed
3. update the owning `all-{group}.md` entrypoint when a group exists
4. run `audit-context`

---

## Repository Structure


```
project-m/
  pc-client/                    -- "Lampyris": Rust desktop hub (TLS client + audio sink + Slint UI)
    src/
      main.rs                   -- clap CLI, tracing setup, Slint MainWindow wiring
      app_state.rs              -- TLS client, SRP auth, adb forward, UI bridge (largest file, 524 loc)
      protocol.rs               -- 'MC' frame parse/emit (the only unit-tested module)
      audio/
        mod.rs                  -- AudioBackend trait + cfg-selected DefaultAudioBackend
        linux.rs                -- PipeWire virtual source sink
        windows.rs              -- IOCTL bridge to the kernel driver
      bin.rs, bin_srp.rs, srp_test.rs   -- small dev/debug binaries
    ui/main.slint               -- Slint UI definition
  android-client/               -- "Sonus": Kotlin + Jetpack Compose mic app
    app/src/main/java/com/projectm/mic/
      MainActivity.kt           -- Compose "Cinema" UI, IP display, pairing PIN
      AudioCaptureService.kt    -- foreground service: TLS server + AudioRecord PCM loop
    app/build/                  -- BUILD ARTIFACTS, committed to git, permanently dirty. Ignore.
    gradle/libs.versions.toml   -- version catalog
  windows-driver/               -- Kernel-mode virtual microphone (C++ / WDM + SYSVAD)
    driver.cpp                  -- WDM driver: IOCTL dispatch + 2s PCM ring buffer
    ioctl.h                     -- shared IOCTL contract with pc-client/src/audio/windows.rs
    lampyris-sysvad/            -- SYSVAD-derived audio miniport (APO, EndpointsCommon, Package, ...)
    *.ps1                       -- cert/signing helper scripts (manual, machine-specific)
    silence_debug_progress.md   -- post-mortem: the four bugs, ruled-out list, runbook
    rebuild_install.ps1         -- build + sign + purge driver store + install (elevated)
  Justfile                      -- root build/test/lint/audit orchestration
  process/                      -- this harness: context, plans, features, protocols
  debug_logs/, graphify-out/    -- scratch output, not source
```

## Technology Stack


- **PC hub (`pc-client/`, crate `lampyris` v1.1.0):** Rust **edition 2024**, async on **tokio 1.52** (`features = ["full"]`), UI in **Slint 1.16** (`slint-build` in build.rs, `slint::include_modules!()`), CLI via **clap 4.6** derive, logging via **tracing 0.1 + tracing-subscriber 0.3** to a file (not stdout).
- **Transport security:** **rustls 0.23** + **tokio-rustls 0.26**, cert generation with **rcgen 0.14**, PEM via **rustls-pemfile 2.2**. A custom `ServerCertVerifier` is implemented in `app_state.rs` (TOFU-style pinning, not webpki roots).
- **Pairing / auth:** **srp 0.7.0-rc.3** and **spake2 0.4**, with **sha2 0.11**, **hex**, and **subtle 2.6** for constant-time comparison.
- **Audio (Linux):** **pipewire 0.10** (plus `pipewire-sys`, `libspa-sys`), gated behind `cfg(target_os = "linux")`.
- **Audio (Windows):** **windows 0.52** crate (`Win32_Storage_FileSystem`, `Win32_System_IO`, `Win32_System_Registry`, `Win32_Foundation`, `Win32_Security`), gated behind `cfg(windows)`. Talks to the kernel driver over `DeviceIoControl`.
- **Buffering:** **ringbuf 0.5**; misc **anyhow 1.0**, **async-trait 0.1**, **dirs 6.0**, **libc 0.2**, **local-ip-address 0.6**, **scopeguard 1.2**.
- **Android app (`android-client/`, `com.projectm.mic` v1.1.0):** **Kotlin 2.1.0**, **AGP 8.4.1**, **Jetpack Compose** (BOM 2024.05.02, Material3), `compileSdk`/`targetSdk` **34**, `minSdk` **26**. Release builds are minified (R8 + `proguard-rules.pro`) and renamed to `Sonus-v{versionName}.apk`.
- **Kernel driver (`windows-driver/`):** C++ **WDM** (`ntddk.h`, `wdmsec.h`) built with MSBuild (`lampyris-mic.sln` / `.vcxproj`), plus a **SYSVAD**-derived audio miniport in `lampyris-sysvad/` (own `sysvad.sln`). Test-signed with PowerShell helper scripts.
- **Build orchestration:** a root **`Justfile`** (`just build-all`, `test-all`, `lint-all`, `audit`, `bench-latency`). There is no npm/pnpm and no `package.json` — this is not a JS project.

## Audio and Wire Contract

These constants are load-bearing across all three components. Changing one without the others breaks the link.

- **Audio format:** 48,000 Hz, 16-bit signed PCM (`s16le`), mono. A `24kHz` variant is signalled by a header flag.
- **Frame layout (protocol v1):** `'M' 'C'` magic, then `ver_flags` (1B: version in the high nibble, flags in the low nibble, bit 0 = `is_24khz`), `seq_num` (2B big-endian), `payload_size` (2B big-endian), then the raw PCM payload. Total header = 7 bytes. `protocol.rs` resyncs by scanning for `'M'`.
- **Flipped architecture:** the **Android device is the TCP/TLS server**; the **PC hub is the client**. This is deliberate — it mimics professional hardware behaviour.
- **Default port:** 47999 (`--port` overrides).
- **IOCTL contract (`windows-driver/ioctl.h`):** device type `0x8001`, function `0x802`, `METHOD_BUFFERED` + `FILE_WRITE_ACCESS`. Payload struct is `{ ULONG Length; BYTE Data[9600]; }`. The driver holds a 2-second ring buffer (`48000 * 2 * 2` bytes) guarded by a `KSPIN_LOCK`.

## Key Patterns and Conventions

**Error handling (Rust):** `anyhow::Result` throughout; `anyhow::anyhow!` for ad-hoc errors. No custom error enum. Failures are logged with `tracing::{error, warn, debug}` and surfaced to the Slint UI as status strings rather than propagated to `main`.

**Platform abstraction:** one trait, `AudioBackend { fn push_samples(&self, samples: &[f32]) }` in `audio/mod.rs`. Backends are selected at compile time with `#[cfg(target_os = ...)]` and re-exported under the single alias `DefaultAudioBackend`. Add a platform by adding a module and a `cfg` re-export — do not add runtime dispatch.

**State:** a single `Arc<AppState>` created in `main.rs` and cloned into each Slint callback (`on_connect_clicked`, `on_disconnect_clicked`) and into the Ctrl+C shutdown task. The UI is held as a `slint::Weak<MainWindow>` and updated via `slint::invoke_from_event_loop`.

**Concurrency:** `#[tokio::main]` drives networking; Slint owns the main event loop via `ui.run()`. Cross-thread UI updates must go through `invoke_from_event_loop`.

**Sample conversion:** `convert_s16_to_f32` and `calculate_peak_amplitude` (bottom of `app_state.rs`) are the single conversion/metering path. Reuse them, do not re-implement.

**Reconnect:** `connect()` retries with `const MAX_RETRIES: u32 = 5`.

**Logging destination:** `pc-client` writes to `{data_local_dir}/lampyris/lampyris.log` (falls back to `/tmp/lampyris.log`), ANSI off. `--debug` raises the level to `DEBUG`. Nothing useful appears on stdout — read the log file when debugging.

**Naming:** Rust `snake_case` modules and files; Kotlin `PascalCase` files matching the class; C++ driver functions prefixed `Lampyris*`. Product names are Lampyris (PC) and Sonus (Android); the repo/package name is `project-m` / `com.projectm.mic`.

**Android:** capture runs in a foreground service (`AudioCaptureService`) with a `microphone` service type; the UI (`MainActivity`) is Compose-only, no XML layouts, one drawable.

**Committed build output:** `android-client/app/build/` is tracked by git and is dirty on essentially every build. Never stage, diff, or "clean up" files under it.

## Environment and Configuration

**There are no environment variables.** Nothing in the codebase reads `env::var` or `getenv`. Configuration is entirely file-based and CLI-flag based.

**Config files:**
- `Justfile` — root task runner
- `pc-client/Cargo.toml`, `pc-client/.cargo/` — Rust build config (incl. cross-compile setup)
- `android-client/gradle/libs.versions.toml` — Android version catalog
- `android-client/local.properties` — git-ignored SDK path
- `windows-driver/lampyris-mic.vcxproj`, `lampyris-sysvad/sysvad.sln` — MSBuild config

**Runtime paths:**
- Certificates / pairing state: `{dirs::config_dir()}/…` (falls back to the current dir)
- Log file: `{dirs::data_local_dir()}/lampyris/lampyris.log` (falls back to `/tmp/lampyris.log`)

**CLI flags (`pc-client`):** `--port <u16>` (default 47999), `--debug`.

**Android permissions requested:** `RECORD_AUDIO`, `INTERNET`, `ACCESS_WIFI_STATE`, `ACCESS_NETWORK_STATE`, `FOREGROUND_SERVICE`, `FOREGROUND_SERVICE_MICROPHONE`, `POST_NOTIFICATIONS`.

**Host dependencies (Linux dev):** `base-devel pkgconf pipewire libpipewire android-tools` (Arch/CachyOS). `adb` is required — the hub shells out to `adb forward` for USB tunnelling.

## Key Files and Source References

The shortest path to understanding each component.

| File | Why it matters |
|---|---|
| `README.md` | product overview, technical specs, getting-started commands |
| `PRODUCT.md` | audience, brand personality, design principles, anti-references |
| `Justfile` | every build / test / lint / audit command |
| `pc-client/src/app_state.rs` | TLS client, custom cert verifier, SRP pairing, adb forward, reconnect, UI bridge |
| `pc-client/src/protocol.rs` | the `'MC'` frame contract, plus the only unit tests in the repo |
| `pc-client/src/audio/mod.rs` | the `AudioBackend` trait — the whole cross-platform seam, 16 lines |
| `windows-driver/ioctl.h` | the IOCTL contract shared between the Rust sender and the C++ driver |
| `windows-driver/driver.cpp` | IOCTL dispatch and the 2-second PCM ring buffer |
| `windows-driver/silence_debug_progress.md` | post-mortem for the capture bugs: the four root causes, the ruled-out list, and the verification runbook |
| `windows-driver/rebuild_install.ps1` | one elevated command for build -> sign -> purge driver store -> install -> reboot |
| `android-client/app/src/main/java/com/projectm/mic/AudioCaptureService.kt` | TLS server + `AudioRecord` capture loop |
| `testing_instructions.md` | manual end-to-end verification steps |

Deeper routing: `process/context/tests/all-tests.md` (verification), `process/context/planning/all-planning.md` (plans), and the three guides under `process/features/`.

## Open Questions and Outstanding Work

- **Windows virtual mic: working and verified (2026-08-21).** Four bugs fixed — stereo-only capture pin, client fake-stereo frames, unbounded ring buffer, and MicIn's unopenable descriptor set. Remaining cosmetic debt: MicIn borrows MicArray's topology, so it carries an inert keyword-detection pin it does not need, and `micintoptable.h` / `micinwavtable.h` are now unused. Neither affects behaviour.
- **`TestApp.exe` is dead code.** It expects `IOCTL_LAMPYRIS_AUTHENTICATE` and `HKLM\SOFTWARE\Lampyris`; the current `ioctl.h` defines only `PUSH_AUDIO` and nothing writes that key.
- **The four APO projects and `KeywordDetectorContosoAdapter` cannot build** — their headers (`DelayAPOInterface.h`, `SwapAPOInterface.h`, `KWSApoInterface.h`, `AecApoDll.h`, `KeywordDetectorOemAdapter.idl`) were never vendored. Build `TabletAudioSample.vcxproj` alone.
- **`windows-driver/driver.cpp` + `lampyris-mic.vcxproj` are dead code.** They build a separate `\Device\LampyrisMic` (no "2") that nothing opens. The live IOCTL device is `\Device\LampyrisMic2`, served by `lampyris-sysvad/lampyris_core.cpp` and consumed in `minwavertstream.cpp` (~line 1541-1542). Do not debug `driver.cpp` for capture-path issues. Deleting this dead code is a candidate future cleanup task, not yet done.
- **No automated test coverage outside `protocol.rs`.** `app_state.rs` (TLS, custom cert verifier, SRP pairing) and both Kotlin files are entirely untested. See `process/context/tests/all-tests.md` for the full gap list.
- **No CI.** Nothing runs on push; there is no `.github/workflows/`.
- **`just bench-latency` has no benchmarks** to run, despite latency being a core product claim.
- **`just audit` needs tooling that is not installed by default** (`cargo deny`, the Gradle dependency-check plugin).
- **Android build output is committed to git** (`android-client/app/build/`), which keeps the working tree permanently dirty. Untracking it is a possible cleanup, not yet decided.

## Scan Metadata

- Generated: 2026-08-21
- HEAD: 9288de4fcb6ada1916e4f02c857f09ff782e2a99 (branch `feat/windows`)
- Mode: fresh
- Package manager: none (cargo + gradle + msbuild, orchestrated by `just`)
