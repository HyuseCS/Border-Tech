---
description: Verification guide for Project-M: which runner to use per component, exact commands, debugging quirks, and the known test-coverage gaps.
keywords: [test, tests, testing, verify, verification, cargo test, gradlew, just, runner, coverage, debug, clippy, lint, benchmark, ci]
---

# Project-M - All Tests

Last updated: 2026-08-21

Attach this file first when the task involves testing, verification, or test debugging.

This is the fast operator guide for the testing surface:

- which runner to use
- what command to start with
- how to quickly debug common failures
- which deeper file to read next

Do not load the whole `process/context/tests/` folder by default. Start here, then drill down.

---

## How This File Works

This is the `all-tests.md` entrypoint for the `tests/` context group. It follows the `all-*.md` routing convention:

1. Agents read `all-context.md` first and get routed here for testing tasks
2. This file gives quick decision rules and commands
3. For deeper details, agents follow the routing table below to specific docs

As the project grows, add deeper docs to this group (e.g., `e2e-tests.md`, `debugging-and-pitfalls.md`) and add routing entries below. This file stays the fast-start entrypoint.

---

## What This Covers

- test runner selection
- quick commands by package
- fast debugging procedures
- current testing gaps worth remembering

## Read This When

Use this file when you need to:

- run tests after implementation
- decide between test runners
- debug failing tests

## Quick Routing


<!-- Example of what a filled-in routing table looks like (from a mature project): -->

<!--
| If you need... | Read next |
|---|---|
| commands and scripts by package | `scripts-and-commands.md` |
| architecture, mocks, auth model, and runner split | `architecture-and-patterns.md` |
| Playwright setup, auth flow, and current specs | `e2e-tests.md` |
| failing-test triage and runtime debugging | `debugging-and-pitfalls.md` |
| known gaps and future test-system fixes | `known-issues.md` |
-->

| If you need... | Read next |
|---|---|
| the current Windows driver bug, evidence, and ruled-out list | `windows-driver/silence_debug_progress.md` |
| driver build failures | `windows-driver/build_errors.md` |
| manual driver capture steps | `windows-driver/debug_capture_instructions.md` |
| manual end-to-end steps | `testing_instructions.md` (repo root) |

(No dedicated deep docs in this context group yet. Add them here as they are created.)

## Quick Decision Guide


<!-- Example of what this looks like filled in (monorepo with Vitest + Bun + Playwright): -->

<!--
### Use `vitest` when

- the change is in React components, hooks, stores, or plugin logic
- the package already has a vitest config and unit-test surface

### Use `bun test` when

- the change is in `packages/api`
- the behavior is router, route, auth-helper, or model logic

### Use Playwright when

- the behavior depends on real navigation, auth redirects, rendering, or full-stack browser flows

### Use container verification when

- the issue is about runtime services, gateway WebSocket behavior, or proxy state
-->

<!-- Example for a simpler single-app project: -->

<!--
### Use `vitest` for everything

- all tests run through vitest
- `vitest run` for CI, `vitest` (watch mode) for development
- Playwright tests also use vitest as the runner via `@playwright/test`
-->

This repo has **three toolchains and almost no automated tests**. Pick by component.

### Use `cargo test` when
- the change is in `pc-client/`
- in practice this means `pc-client/src/protocol.rs` — the `'MC'` framing module is the **only** module with unit tests (5 `#[tokio::test]` cases in a `#[cfg(test)]` block)
- run it for any protocol, framing, or sequence-number change; it is fast and it is the one real safety net in the repo

### Use `cargo build` / `cargo clippy` as the verification step when
- the change is in `app_state.rs`, `audio/linux.rs`, or `audio/windows.rs` — there are no tests here, so a clean compile plus manual run is the bar
- **always check both platforms.** The audio backends are `#[cfg]`-gated, so a Linux-only build never compiles `audio/windows.rs` and vice versa. A green build on one host proves nothing about the other.

### Use `./gradlew test` when
- the change is in `android-client/`
- **be aware:** there is no `src/test/` or `src/androidTest/` directory. The task runs and passes with zero tests. It is a compile check, not a test.

### Use a manual run on real hardware when
- the change touches audio capture, latency, PipeWire routing, or the Windows driver
- this is currently the only way to verify the end-to-end path. See `testing_instructions.md` at the repo root.

### Use a clean full-solution rebuild when
- the change is in `windows-driver/`
- a single-project build has already caused a stale `EndpointsCommon.lib` to be relinked, producing a fake `0xC00000BB` failure that cost a debugging detour. Rebuild the whole solution, every time.

## Default Verification Order

Unless the task clearly needs a different path:

1. run the narrowest existing automated test
2. use unit/integration tests before browser tests
3. use end-to-end tests only when the real UI is the thing being verified

## Commands


<!-- Example of what this looks like filled in (monorepo): -->

<!--
| Package | Runner | Command | Notes |
|---|---|---|---|
| `apps/web` | vitest | `pnpm --filter web test` | jsdom environment |
| `packages/api` | bun test | `pnpm --filter @acme/api test` | needs `.env.test` |
| `packages/db` | bun test | `pnpm --filter @acme/db test` | needs running database |
| `apps/web` (e2e) | Playwright | `pnpm --filter web test:e2e` | needs dev server running |
| root | all | `pnpm test` | runs all packages |

**Typecheck (not a test runner, but often needed for verification):**
```bash
pnpm typecheck          # all packages
pnpm --filter web typecheck  # single package
```

**Lint:**
```bash
pnpm lint               # all packages
pnpm lint:verified      # lint + typecheck together
```
-->

<!-- Example for a simpler single-app project: -->

<!--
```bash
pnpm test              # run all tests (vitest)
pnpm test:e2e          # run Playwright e2e tests
pnpm test -- --watch   # watch mode
pnpm test -- path/to/file.test.ts  # single file
```
-->

All root commands are wrapped by the `Justfile`.

| Component | Runner | Command | Notes |
|---|---|---|---|
| `pc-client/` | cargo | `cd pc-client && cargo test` | 5 real tests, all in `protocol.rs` |
| `android-client/` | Gradle | `cd android-client && ./gradlew test` | **no test sources exist** — compile check only |
| `windows-driver/` | — | none | no automated tests; manual verification only |
| all | just | `just test-all` | runs `cargo test` + `./gradlew test` |

**Build:**
```bash
just build-all                  # cargo build --release + ./gradlew assembleRelease
cd pc-client && cargo build     # debug build, faster
```

**Lint / format:**
```bash
just lint-all                   # cargo fmt --check + cargo clippy -D warnings + ./gradlew lint
```

**Dependency audit:**
```bash
just audit                      # cargo deny check + ./gradlew dependencyCheckAnalyze
```

**Benchmarks:**
```bash
just bench-latency              # cd pc-client && cargo bench
```

**Run the hub:**
```bash
cd pc-client && cargo run --release -- --port 47999 --debug
```

## Debugging Quick Reference

- **Logs are not on stdout.** `pc-client` writes everything to `{data_local_dir}/lampyris/lampyris.log` (fallback `/tmp/lampyris.log`), ANSI off. When a run "prints nothing", read that file. Pass `--debug` to raise the level to `DEBUG`.
- **`cfg`-gated code does not compile on the other host.** Verify `audio/windows.rs` changes on Windows (or cross-compile with the MinGW target) and `audio/linux.rs` changes on Linux. Clippy on one host will not catch errors in the other module.
- **`just audit` needs extra tooling.** `cargo deny` and the Gradle `dependencyCheckAnalyze` plugin must be installed; neither is a default.
- **`just bench-latency` has no benches yet.** `cargo bench` currently has nothing to run.
- **Windows driver: always rebuild the full solution.** Single-project builds relink stale `.lib` files and produce misleading failures.
- **Android build output is committed to git.** `android-client/app/build/` changes on every build and will pollute `git status` and `git diff`. Ignore it; never stage it.
- **The hub needs `adb` on PATH** for USB mode — it shells out to `adb forward`.
- **Linux runtime needs PipeWire running**, not just installed.

<!-- Example of what this looks like filled in: -->

<!--
- **jsdom quirks:** `apps/web` uses jsdom -- Canvas/Image APIs unavailable, mock them in test setup
- **env files:** `packages/api` tests require `.env.test` with `DATABASE_URL` pointing to test DB
- **database state:** API tests use PGlite for isolated test databases, no external DB needed
- **auth mocking:** Clerk is mocked via `vi.mock("@clerk/nextjs")` in web tests
- **port conflicts:** dev server must be stopped before running e2e tests (both use port 3000)
-->

## Known Gaps

These are known and accepted. Do not assume a test exists.

- **No Android tests at all.** No `src/test/`, no `src/androidTest/`. `AudioCaptureService.kt` (550 loc) and `MainActivity.kt` (485 loc) are entirely unverified by automation.
- **No tests for `app_state.rs`** (524 loc) — the TLS client, custom cert verifier, SRP pairing, adb forwarding, and reconnect logic are all untested.
- **No tests for either audio backend** (`linux.rs`, `windows.rs`).
- **No tests for the kernel driver.** `driver.cpp`, the IOCTL dispatch, and the ring buffer are verified only by running the driver on a real or virtual Windows host.
- **No end-to-end / integration test.** The Android → TLS → PC → OS-audio path is verified only by hand.
- **No CI.** There is no `.github/workflows/`; nothing runs automatically on push.
- **No benchmarks**, despite the `bench-latency` target existing.

<!-- Example: -->

<!--
- No integration tests for the billing webhook handler
- E2E tests do not cover the admin dashboard (only the main web app)
- Container runtime tests require manual Docker setup (not in CI yet)
-->
