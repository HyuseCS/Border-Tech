---
name: plan:connection-resilience
description: "Close GitHub issues #1-#3: PC-side stall detection, Android teardown race, notification stop, watchdog disconnect, and handshake/accept hardening"
date: 21-08-26
feature: connection-resilience
---

# Connection Resilience — Plan

Closes GitHub issues #1, #2, #3 on `HyuseCS/Border-Tech`. Issue #4 already closed. INNOVATE is
complete — this plan is the exact executable checklist for the locked design below. No design
decisions are re-litigated here.

**Date**: 21-08-26
**Status**: DRAFT — pending VALIDATE
**Complexity**: COMPLEX

## Overview

Two independent, non-overlapping phases:

- **Phase 1 (PC / Rust):** stop the PC client's VU meter and status text from silently freezing
  when the phone-side connection stalls, by timing out the frame-header read the same way the
  payload read is already timed out.
- **Phase 2 (Android / Kotlin):** fix a structural teardown race, make the notification a
  discoverable one-tap disconnect, add a real cross-thread watchdog that can unblock a
  stuck socket write, and harden the accept/handshake loop so one bad peer can't wedge the
  whole server.

## Goals

- Issue #1: PC UI (VU meter + status text) must visibly reflect a stalled/dead connection
  instead of freezing at the last good frame.
- Issue #2: Android side must detect a "silent death" (blocked `write()` for minutes) and
  recover, without a teardown race between the read loop and the client-disconnect path.
- Issue #3: The persistent notification must offer a discoverable, working way to stop the
  connection.

## Scope

In scope: `pc-client/src/protocol.rs`, `pc-client/src/app_state.rs`, `pc-client/ui/main.slint`,
`android-client/app/src/main/java/com/projectm/mic/AudioCaptureService.kt`.

Explicitly out of scope (rejected in INNOVATE — do not implement):
- socket2 / TCP keepalive tuning on either side
- any application-level heartbeat frame
- any change to the `'MC'` wire format
- a new PC-side connection-state enum (existing `is_connected: bool` + `status_text` stays)
- a Stop action button on the Android notification, or wiring the notification body to open `MainActivity`
- crypto/SRP/TLS handshake logic changes (only a socket timeout wraps the existing handshake reads)
- `START_NOT_STICKY` change in `AudioCaptureService`
- building Android test infrastructure (no `src/test/`, no `src/androidTest/` exists — do not add it)

## Touchpoints

| File | Lines (current) | Change |
|---|---|---|
| `pc-client/src/protocol.rs` | `read_next_frame`, magic-byte read at the start of the scan loop (currently `self.stream.read_exact(&mut header[0..1]).await?;`, immediately before the existing `if header[0] == b'M'` check) | Wrap in `tokio::time::timeout(Duration::from_secs(5), ...)`, mirroring the existing payload-read timeout pattern a few lines below (`Duration::from_secs(2)` around the payload `read_exact`). On elapse, return `anyhow::anyhow!("Timeout waiting for frame header")` so the existing caller error path treats it exactly like any other read error. |
| `pc-client/src/protocol.rs` | `mod tests` (bottom of file, after `test_sequence_gap_silence_insertion`) | Add one new `#[tokio::test]` using the existing `tokio_test::io::Builder` harness for the stalled-header case (see Verification section). |
| `pc-client/src/app_state.rs` | stream loop `match res { Ok(0) => .., Ok(n) => { .. sets `volume_level` .. }, Err(e) => return Err(..) }` (the `Err(e)` arm currently only returns the error, at the tail of the block shown around `Err(e) => { return Err(anyhow::anyhow!("Stream read error: {}", e)); }`) | On `Err(e)` (which now also fires on header-read timeout), before propagating: push `ui.set_volume_level(0.0)` via the same `slint::invoke_from_event_loop` pattern already used in the `Ok(n)` arm, so the meter drops to zero instead of freezing at its last peak. |
| `pc-client/src/app_state.rs` | reconnect/status-update call sites around the existing retry loop (`connect()`, `const MAX_RETRIES: u32 = 5`) and wherever `status_text` is currently set on disconnect/retry | Update the `status_text` string set on a stream-loop error / retry attempt to read something like `"Connection lost, reconnecting..."` (or match whatever short phrase is already used elsewhere in that function — do not invent new phrasing patterns, reuse the existing string-setting call shape). |
| `pc-client/ui/main.slint` | `in-out property <bool> is_connected: false;` / `in-out property <string> status_text: "Disconnected";` (existing properties, lines ~147-149) | No structural change — these two existing properties are sufficient to carry the new status text and the zeroed meter. Do not add a new enum/state property. |
| `android-client/.../AudioCaptureService.kt` | `private var audioRecord: AudioRecord?` field declaration (:94) | Replace with `private val audioRecord = AtomicReference<AudioRecord?>(null)`. Add `import java.util.concurrent.atomic.AtomicReference`. |
| `android-client/.../AudioCaptureService.kt` | per-client `finally` block that currently reads/nulls `audioRecord` (:419-424) | Replace direct field access with `audioRecord.getAndSet(null)?.let { it.stop(); it.release() }` — wrapped in existing try/catch style already used at that call site (match existing exception-swallow style; do not introduce new exception types). |
| `android-client/.../AudioCaptureService.kt` | `cleanup()` (:518-531) | Same `getAndSet(null)?.let { it.stop(); it.release() }` pattern, replacing the current field read/release. |
| `android-client/.../AudioCaptureService.kt` | remaining direct WRITE to the `audioRecord` field: `audioRecord = recorder` (:351), immediately after the `AudioRecord` is constructed in `handleClient` | Update to `audioRecord.set(recorder)`. This is the only remaining field access after items 7-9 (verified by grepping every `audioRecord` occurrence: :94 decl, :351 write, :422-424 finally, :521-526 `cleanup()` — exactly 8 sites, all now covered). **Do NOT** touch the capture loop's `recorder.read()` call (:368) — it reads the LOCAL `val recorder`, not the field, and must stay that way (see item 10). |
| `android-client/.../AudioCaptureService.kt` | `createNotification()` (:138-147) | Add `.setContentIntent(PendingIntent.getService(this, 0, stopIntent, PendingIntent.FLAG_IMMUTABLE))` where `stopIntent` is a new `Intent(this, AudioCaptureService::class.java).setAction(ACTION_STOP)`. Update the notification text/content line to include a short "Tap to stop" hint. Keep `.setOngoing(true)` unchanged. |
| `android-client/.../AudioCaptureService.kt` | `onStartCommand()` (:107) and its `companion object` (wherever action constants live, adjacent to existing service actions) | Add `const val ACTION_STOP = "..."` (follow existing action-constant naming style) and an `if (intent?.action == ACTION_STOP) { stopService(); return START_NOT_STICKY }`-shaped branch (match the existing `stopService()` path at :85-88 exactly — do not write a second stop implementation). |
| `android-client/.../AudioCaptureService.kt` | write loop that calls `sendAudioPacket(...)` and then computes `val duration = System.currentTimeMillis() - startTime` (:380-392) | Add a `@Volatile private var lastSuccessfulWriteMs: Long = System.currentTimeMillis()` field; update it immediately after each successful `sendAudioPacket(...)` call returns (i.e. right where the loop currently proceeds to the `duration` measurement). Do not alter the existing `duration`/`writeStallsCount` logic or the 24kHz degradation branch at :393-405 — they stay exactly as-is. |
| `android-client/.../AudioCaptureService.kt` | new code inside `handleClient(socket)` (:213), scoped to the individual client connection — **not** near :117, which is in `onStartCommand` and only knows about the server-loop job, not any per-client socket | Launch a watchdog coroutine (`serviceScope.launch(Dispatchers.IO) { ... }`) INSIDE `handleClient`, stored as a local `val watchdogJob`, right before the streaming loop begins (~:366). It polls `lastSuccessfulWriteMs` on an interval and, if `System.currentTimeMillis() - lastSuccessfulWriteMs` exceeds the watchdog threshold, calls `socket.close()` on this connection's client socket to unblock the stuck `write()`. Cancel `watchdogJob` explicitly in `handleClient`'s own `finally` block (:419-431), alongside the `audioRecord` cleanup — see item 20. |
| `android-client/.../AudioCaptureService.kt` | SRP/auth read section (:234-248, currently no timeout) | Wrap the handshake reads in `socket.soTimeout = HANDSHAKE_TIMEOUT_MS` (set before the handshake reads begin, e.g. right after `socket.accept()`/before the SRP exchange) — restore/clear `soTimeout` afterward if the surrounding code reuses the socket for the streaming phase with a different timeout expectation. Do NOT touch the SRP/TLS handshake logic itself, only the timeout wrapping it. |
| `android-client/.../AudioCaptureService.kt` | `accept()` failure handling (:193-196, currently `break`s the loop) | Replace the `break` with a `continue` (retry) guarded against a hot-spin loop — e.g. a short `delay(...)` or a bounded consecutive-failure counter before genuinely giving up, matching existing coroutine idioms in the file. |
| `android-client/.../AudioCaptureService.kt` | exception-swallow site (:417-418) and the `finally` that resets to `CONNECTING` (:426-430) | On the caught exception, explicitly set state to `ERROR` (or the new state value, see below) before the `finally` runs, instead of letting it fall through unset. |
| `android-client/.../AudioCaptureService.kt` | `ConnectionState` enum (:42-47) | Reuse the existing `ERROR` value if it already fits the "link just dropped" case; otherwise add **at most one** new enum value (e.g. `DISCONNECTED` or `IDLE`) to distinguish "listening for first connection" from "link just dropped". This is a one-value fix to an existing enum — not a new state model. |
| `android-client/.../MainActivity.kt` | the TWO exhaustive `when (connectionState)` expressions at :130-135 (`statusColor`) and :188-193 (status text) — **not** :88-96, which is only `val connectionState by AudioCaptureService.state` (a plain property read, not a `when`) and needs no branch | Kotlin requires a `when` used as an expression to be exhaustive, and neither site has an `else`. Adding any new `ConnectionState` enum value is therefore a **hard compile error at both sites** until both are updated — there is no "fallthrough" to reason about; the compiler will not let a caller be forgotten. Add a matching branch to both. (Two non-exhaustive comparison sites also exist, :261 and :267, using `!=`/`==` against `ERROR` — these do NOT need updating; they silently keep working with a new value, informational only.) |

## Public Contracts

None of this touches the `'MC'` wire protocol, the IOCTL contract, or any cross-process contract.
The only "contract" surfaces touched are internal:
- PC: `AppState` → Slint UI properties (`is_connected`, `status_text`, `volume_level`) — existing shape, values only.
- Android: `ConnectionState` enum consumed by `MainActivity` — additive at most one new value.
- Android: notification `PendingIntent` action contract (`ACTION_STOP`) is new but internal to this service (not exposed externally).

No external API, schema, or auth/crypto behavior changes.

## Blast Radius

- **Files touched:** 4 (`protocol.rs`, `app_state.rs`, `main.slint`, `AudioCaptureService.kt`), plus a possible small conditional touch to `MainActivity.kt` (0-3 line branches, only if a fallthrough is misleading).
- **Risk class flag (per `all-context.md` High-Risk Areas):** this plan touches **two of the four** documented high-risk areas:
  - **Android capture loop** (`AudioCaptureService.kt`) — directly in scope; this is the single largest touchpoint set in the plan (teardown race, watchdog, accept loop, notification).
  - **TLS + SRP auth path** (`app_state.rs` / `AudioCaptureService.kt`) — adjacent, not modified in substance. The PC side gets a header-read timeout (not touching TLS/SRP at all). The Android side adds `socket.soTimeout` *around* the existing SRP handshake reads — the crypto/handshake logic itself is untouched, only the timeout wrapping changes. State this plainly: neither issue changes crypto, but both edits sit next to the auth path and must be reviewed with that adjacency in mind.
  - Not touched: **Windows kernel driver**, **audio backends** (`pc-client/src/audio/`) — no change to either.
- **Phases 1 and 2 have zero file overlap and no build dependency — they are parallel-safe.** Phase 1 is entirely `pc-client/` + `pc-client/ui/`; Phase 2 is entirely `android-client/`. Neither phase's code path is imported or built by the other. The Linux dev host can build and test `pc-client` for Linux (`cargo build`, `cargo test`) and can build/test `android-client` (`./gradlew assembleDebug`, `./gradlew test`) — **neither phase needs the Windows VM.** They may be executed in either order or concurrently.

## Implementation Checklist

### Phase 1 — PC (Rust) — `pc-client/`

1. In `pc-client/src/protocol.rs`, `read_next_frame`: wrap the magic-byte scan read
   (`self.stream.read_exact(&mut header[0..1]).await?;`) in
   `tokio::time::timeout(std::time::Duration::from_secs(5), self.stream.read_exact(&mut header[0..1])).await`,
   matching the `match ... { Ok(res) => { res?; } Err(_) => { ... return Err(...) } }` shape already
   used for the payload read a few lines below. On timeout, `error!(...)` and
   `return Err(anyhow::anyhow!("Timeout waiting for frame header"))`.
2. In `pc-client/src/app_state.rs`, on the stream-loop `Err(e)` arm (currently just
   `return Err(anyhow::anyhow!("Stream read error: {}", e));`): before returning, push
   `ui.set_volume_level(0.0)` through the same `slint::invoke_from_event_loop` closure pattern used
   in the `Ok(n)` arm, so the VU meter drops to zero instead of freezing at its last peak.
3. In `pc-client/src/app_state.rs`, update the `status_text` string that gets set when the stream
   loop errors / a reconnect attempt begins, to something like `"Connection lost, reconnecting..."`
   — reuse the existing call shape that sets `status_text` elsewhere in the same function; do not
   introduce a new update mechanism.
4. Add a code comment at the timeout constant recording the 5s justification: measured frame
   cadence is ~10ms (48000Hz / 480-sample buffer, confirmed in `AudioCaptureService.kt`), so 5s is
   ~500x the normal inter-frame gap; chosen as a UX threshold for "how long before a frozen VU
   meter reads as broken," deliberately loose enough to survive a GC pause, USB re-enumeration, or
   a Wi-Fi roam.
5. Add `#[tokio::test(start_paused = true)] async fn test_header_read_timeout()` to the existing
   `mod tests` block in `protocol.rs`. Build the mock via
   `Builder::new().wait(std::time::Duration::from_secs(10)).build()` (a wait longer than the 5s header
   timeout). **Do NOT** use a bare `Builder::new().build()` with no actions and expect it to "hang": since
   `.build()` discards the mock's `Handle`, its internal action channel closes immediately and the read
   returns EOF instantly instead of stalling — that trips `read_exact`'s `UnexpectedEof` path, not the
   `tokio::time::timeout` path under test, and the assertion below would fail. Assert `read_audio_packet`
   returns `Err` whose message contains `"Timeout waiting for frame header"`. `start_paused = true` is
   required (confirmed compatible: `tokio = { features = ["full"] }` already pulls in `test-util`, and
   `tokio_test::io::Builder`'s `wait()` uses `tokio::time::sleep_until` internally, which honours the
   paused/auto-advancing virtual clock) — without it this test would burn a real 5-second wall-clock sleep
   in the repo's only test suite. This is a REQUIRED deliverable, not optional — see Verification Evidence.
6. Confirm `is_connected` / `status_text` bindings in `pc-client/ui/main.slint` need no structural
   change — the existing `in-out property <bool> is_connected` and
   `in-out property <string> status_text` already carry everything Phase 1 needs.

### Phase 2 — Android (Kotlin) — `android-client/app/src/main/java/com/projectm/mic/AudioCaptureService.kt` ONLY

Execute strictly in this order — 2a must land before 2b, 2b before 2c, per the locked design.

**2a — Teardown race (issue #2 structural fix)**

7. Replace `private var audioRecord: AudioRecord?` (:94) with
   `private val audioRecord = AtomicReference<AudioRecord?>(null)`; add the
   `java.util.concurrent.atomic.AtomicReference` import.
8. Update the per-client `finally` block (:419-424) to
   `audioRecord.getAndSet(null)?.let { it.stop(); it.release() }`, preserving the existing
   try/catch/exception-swallow style already at that call site.
9. Update `cleanup()` (:518-531) to the same `getAndSet(null)?.let { it.stop(); it.release() }`
   pattern.
10. Update the one remaining direct WRITE to the `audioRecord` field — `audioRecord = recorder` (:351,
    immediately after the `AudioRecord` is constructed in `handleClient`) — to `audioRecord.set(recorder)`.
    **Correction:** grepping every `audioRecord` occurrence in the file shows exactly 8 sites: :94 (decl,
    item 7), :351 (write — this item), :422-424 (per-client `finally`, item 8), :521-526 (`cleanup()`,
    item 9). The capture loop's `recorder.read()` call (:368) does **NOT** touch the field — it reads the
    LOCAL `val recorder` assigned at :344-351, and must stay that way; do not convert it to
    `audioRecord.get()`. The local reference is load-bearing for the teardown/unblock mechanism: when
    another thread calls `audioRecord.getAndSet(null)?.let { it.stop(); it.release() }`, the loop's local
    `recorder` still points at the now-released `AudioRecord`; the next `recorder.read(...)` throws
    `IllegalStateException`, which is caught by the surrounding `catch (e: Exception)` at :417-418 and ends
    the loop cleanly — that fail-fast is exactly what makes cross-thread release effective. Routing the loop
    through `audioRecord.get()` instead would yield `null` and silently skip reads (a spin), turning a clean
    fail-fast into a stall. Add a short code comment at the loop stating this, per the plan's own
    "document why" convention, so a future reader does not "fix" the local back into a field read.

**2b — Notification stop (issue #3, per user's explicit product decision)**

11. In `createNotification()` (:138-147), add a `stopIntent = Intent(this, AudioCaptureService::class.java).setAction(ACTION_STOP)` and attach it via
    `.setContentIntent(PendingIntent.getService(this, 0, stopIntent, PendingIntent.FLAG_IMMUTABLE))`.
    `FLAG_IMMUTABLE` is required at targetSdk 34.
12. Update the notification's text/content line to include a short discoverability hint (e.g. a
    "Tap to stop" line), since tapping the body is now destructive and there is no separate Stop
    button.
13. Keep `.setOngoing(true)` unchanged — swipe-to-dismiss stays disabled; only tapping stops it.
14. Add `const val ACTION_STOP = "..."` alongside existing action constants (match existing naming
    style in the companion object).
15. In `onStartCommand()` (:107-123), add an `if (intent?.action == ACTION_STOP)` branch that routes
    into the **existing** `stopService()` path (:85-88) — do not write a second/duplicate stop
    implementation. **Ordering is mandatory, not cosmetic:** `onStartCommand` currently ALWAYS runs
    `startForegroundNotification()` (:112), sets `state.value = CONNECTING` (:114), cancels `captureJob`
    and relaunches `runServerLoop` (:117-120) before returning `START_NOT_STICKY` (:122). The `ACTION_STOP`
    check MUST be the very first statement in the function body (before reading the `port`/`isUsb` extras)
    and MUST `return START_NOT_STICKY` immediately after calling `stopService(this)` — otherwise a stop
    intent would re-notify, flip state back to CONNECTING, and relaunch the server it was sent to stop.
    Do NOT wire the notification body to open `MainActivity` and do NOT add a separate Stop action
    button — both explicitly rejected.
16. Do not proceed to 2c until 2a and 2b are both landed and building — notification taps can
    arrive at arbitrary moments and make the teardown race in 2c far easier to hit if 2a isn't in
    place first.

**2c — Stall-triggered disconnect watchdog (issue #2 core)**

17. Add `@Volatile private var lastSuccessfulWriteMs: Long = System.currentTimeMillis()` as a
    service-level field.
18. In the write loop, immediately after `sendAudioPacket(...)` returns successfully (before or
    alongside the existing `duration` measurement at :380-392), update
    `lastSuccessfulWriteMs = System.currentTimeMillis()`.
19. **Correction (checklist item was not implementable as written — `:117` is the wrong scope):**
    `:117` (`captureJob?.cancel()` / relaunch) lives in `onStartCommand`, which only knows about the
    SERVER-LOOP job (`runServerLoop`, the accept loop) — it has no reference to any individual client
    socket. The socket the watchdog must close is the `socket: Socket` parameter LOCAL to
    `handleClient(socket)` (:213); it does not exist at :117. Launch the watchdog coroutine
    (`serviceScope.launch(Dispatchers.IO) { ... }`) **INSIDE `handleClient`**, scoped to this one client
    connection, storing its `Job` in a local `val watchdogJob` declared right before the streaming loop
    begins (~:366, after `recorder.startRecording()` and before `while (isServiceRunning.value && ...)`).
    Immediately before launching it, reset `lastSuccessfulWriteMs = System.currentTimeMillis()` — since
    that field is service-level (shared across sequential client connections), a stale value left over from
    a PREVIOUS connection's last write (or the service's `onCreate` init time, if no client has connected
    yet) would otherwise make the watchdog force-close a brand-new socket before it ever gets a chance to
    write its first frame. The watchdog polls on an interval and, when
    `System.currentTimeMillis() - lastSuccessfulWriteMs` exceeds the watchdog threshold, calls
    `socket.close()` on THIS connection's client socket. This is the mechanism that unblocks a thread
    parked inside a blocked `write()` — closing the socket from another thread, NOT `captureJob.cancel()`,
    which only trips cooperative coroutine cancellation and does nothing to a thread already blocked
    inside a blocking Java call (nor would it reach this watchdog at all, since it is not parented under
    `captureJob` — see item 20).
20. **Correction:** since the watchdog is scoped per-connection inside `handleClient` (item 19), it is a
    sibling of `captureJob`, not a child of it — `captureJob?.cancel()` (:117, :544) does NOT cancel it.
    Cancel `watchdogJob` explicitly with `watchdogJob.cancel()` in `handleClient`'s own `finally` block
    (:419-431), alongside the `audioRecord` cleanup (item 8), so it is torn down every time this specific
    client connection ends — whether by graceful disconnect, read/write error, or the watchdog's own
    `socket.close()` — and never outlives the connection it is guarding.
21. Pick and document the watchdog threshold in a code comment, placed next to the existing 50ms
    stall / 24kHz-degradation threshold, explicitly contrasting the two:
    - Existing 50ms threshold: codec-quality heuristic — triggers 24kHz degradation, NOT a
      liveness signal. Leave its behavior (:393-405) completely unchanged.
    - New watchdog threshold: liveness signal — must be clearly above the 50ms threshold so
      normal backpressure never trips a disconnect. **Recommend 5s, not the low end of a 3-5s
      range**: the existing degrade-to-24kHz safety valve (:392-410) already absorbs ordinary Wi-Fi
      jitter by reducing bandwidth after 3 consecutive >50ms writes, so legitimate congestion is
      usually resolved well before a multi-second stall; picking 5s (vs. 3s) leaves more headroom
      against scheduling jitter and false positives while still recovering promptly. This is
      coincidentally the same order of magnitude as the PC-side 5s header-read timeout from Phase 1,
      but the two remain intentionally uncoupled (see Dependencies/Risks) — pick and justify the exact
      number in the code comment at implementation time based on observed normal jitter, but do not go
      below 5s without a measured reason.
    - Comment must state both numbers side by side so a future reader does not have to
      reverse-engineer why they differ.

**2d — Handshake/accept hardening (also issue #2 evidence, each its own item)**

22. Set `socket.soTimeout = HANDSHAKE_TIMEOUT_MS` as the FIRST statement inside `handleClient(socket)`
    (:213), before the lockout check (:215). **This is a wider scope than ":234-248" alone** — the
    SRP/auth reads at :234-248 are not the only blocking read to protect: `socket.startHandshake()`
    (:221-225, the TLS handshake) is itself a blocking read on the same socket and is exactly the kind of
    slow/malicious-peer hang issue #2 exists to close. Because `soTimeout` is a persistent socket property,
    setting it once at the top of `handleClient` covers the TLS handshake AND every SRP read (:234, :245,
    :283, :293) with no need to re-set it at each call site. Confirmed safe on `SSLSocket`: `SSLSocket`
    extends `Socket` and inherits `setSoTimeout`/`getSoTimeout` from it (import already present at the top
    of the file) — a `SocketTimeoutException` thrown mid-handshake propagates to the general
    `catch (e: Exception)` at :417 exactly like any other handshake failure, and the existing `finally`
    block (:419-431) already closes the socket unconditionally, so no special-case handling is needed.
    **Clear it before streaming starts** — `socket.soTimeout = 0`, right after authentication succeeds
    (~:319, before the "Audio Record configuration" section at :335). Note the streaming loop (:367-415)
    only ever calls `outputStream`/`sendAudioPacket` (writes) — it never reads `inputStream` again, and
    `soTimeout` only affects blocking `read()` calls, never writes. So a lingering handshake `soTimeout`
    would be **inert** during streaming, not silently breaking it — but clear it anyway as defensive
    hygiene, in case a future change adds a read to the streaming phase. Do NOT alter the SRP/TLS handshake
    logic itself — crypto stays exactly as-is; only the timeout wrapping is new.
23. Replace the `accept()` failure `break` (:193-196) with a retry (`continue`), guarded against a
    hot spin loop (e.g. a short delay or a bounded consecutive-failure counter before genuinely
    giving up), matching existing coroutine idioms already used in the file.
24. At the exception-swallow site (:417-418), explicitly set connection state to `ERROR` (or the
    new enum value from item 25) before the `finally` at :426-430 runs, instead of letting the
    state fall through unset. **Also update the `finally` block's condition** (:426-430 currently reads
    `if (isServiceRunning.value) { state.value = ConnectionState.CONNECTING }` unconditionally) to skip
    the CONNECTING overwrite when this catch block just set `ERROR` — e.g.
    `if (isServiceRunning.value && state.value != ConnectionState.ERROR) { state.value = CONNECTING }`,
    mirroring the existing guard already used in `cleanup()` (:534, `if (state.value != ConnectionState.ERROR)`).
    Without this, the `finally` block runs immediately after the catch block in the same
    `serviceScope.launch(Dispatchers.Main)` queue and unconditionally overwrites `ERROR` back to
    `CONNECTING`, silently negating this item's fix.
25. In `ConnectionState` (:42-47): reuse the existing `ERROR` value if it fits "the link just
    dropped"; otherwise add **at most one** new enum value to distinguish "listening for a first
    connection" from "the link just dropped." This is a one-value fix to an existing misused enum
    — not a new state model.
26. **Correction (was understated — this is a compile-time requirement, not a judgment call):**
    `MainActivity.kt` has exactly two `when (connectionState)` expressions, at :130-135 (`statusColor`)
    and :188-193 (status text) — both used as expressions with no `else` branch. `:88-96` is NOT one of
    them (it is `val connectionState by AudioCaptureService.state` at :88, a plain property read — the
    stale reference from an earlier research pass; drop it). Kotlin requires a `when` used as an
    expression to be exhaustive, so adding any new `ConnectionState` enum value is a **hard compile
    error at both :130-135 and :188-193 until both are updated** — there is no fallthrough to reason
    about. This is a point in favour of adding a new value rather than overloading `ERROR`: the compiler
    will not let a caller be forgotten. Add the matching branch to both sites. (Two non-exhaustive
    comparison sites also reference `connectionState`, at :261 and :267, using `!=`/`==` against `ERROR`
    — these compile fine either way and need no change; noted for completeness only.)

## Do-Not-Regress Checks

Explicitly re-verify these paths still work after Phase 2 — issue #1's investigation confirmed
they were NOT broken, and this plan must not break them:

1. Graceful disconnect (user-initiated stop) still tears down cleanly — no double-release, no
   orphaned socket.
2. Both audio backends' silence-on-underrun behavior (`pc-client/src/audio/linux.rs` and
   `windows.rs`) is unaffected — Phase 1 does not touch `audio/`.
3. No task double-start on reconnect — verify the watchdog coroutine and the capture job are never
   both left running after a reconnect (item 20 covers this structurally).
4. Ctrl+C shutdown on the PC client still exits cleanly (this is a `pc-client/src/main.rs`
   concern, unaffected by Phase 1's `protocol.rs`/`app_state.rs` edits, but must be re-checked
   manually since `app_state.rs` is touched).

## Build Hygiene (mandatory, repo rules)

- `android-client/app/build.gradle.kts` uses **CRLF** line endings. Do not touch it in this plan,
  but as a general rule for any file touched: **preserve existing line endings** — an editor that
  rewrites CRLF to LF turns a small diff into a huge one. Verify with
  `file android-client/app/src/main/java/com/projectm/mic/AudioCaptureService.kt` and
  `git diff --stat` before committing that the diff size matches the actual number of changed
  lines.
- Never stage anything under `android-client/app/build/` (tracked-but-generated, permanently
  dirty). Never run `git add -A` in this repo — stage explicit paths only.
- Minimum code that solves the problem. No speculative abstraction (no generic "watchdog
  framework," no configurable timeout system nobody asked for — hardcoded constants with
  justification comments are correct here). No unrequested error-handling for scenarios that
  can't occur. Surgical changes only — do not reformat or "improve" adjacent code. Match existing
  style exactly (Kotlin idioms already in the file, Rust idioms already in `protocol.rs`/`app_state.rs`).

## Verification Evidence

| Gate / Scenario | Strategy | Proves SPEC criterion |
|---|---|---|
| `cargo test test_header_read_timeout` in `pc-client/src/protocol.rs::tests` (new test, `tokio_test::io::Builder` mock that never/late-delivers the header byte) | Fully-Automated | Issue #1 core: header read no longer blocks forever; times out and surfaces as an `Err` the existing retry loop already handles |
| `just test-all` → `cd pc-client && cargo test` (full existing `mod tests` suite, all 5 prior + 1 new test) | Fully-Automated | Regression: existing frame-parse, resync, overflow-protection, degraded-mode, and sequence-gap behavior in `protocol.rs` is unchanged |
| `just lint-all` → `cargo fmt --check && cargo clippy -- -D warnings` (pc-client) | Fully-Automated | Code style / lint regression guard on Phase 1 changes |
| Manual: kill the PC client mid-stream (Ctrl+C or `kill -9` the phone-side app while streaming) | Agent-Probe (manual, human-executed) | Issue #1: VU meter visibly drops to zero and `status_text` shows a reconnecting message within ~5s of the kill, instead of freezing at the last peak |
| Manual: cut Wi-Fi mid-stream (disable Wi-Fi on the phone or the PC host while streaming) | Agent-Probe (manual, human-executed) | Issue #1 + #2: PC UI reflects the stall within ~5s; Android side's watchdog closes the stuck socket within the documented watchdog threshold (recommend testing at ~2x the chosen threshold to allow for scheduling jitter) |
| Manual: tap the notification body mid-stream | Agent-Probe (manual, human-executed) | Issue #3: connection stops immediately, service tears down cleanly (no crash, no double-release — confirms 2a landed correctly under 2b's arbitrary-timing pressure) |
| Manual: open a raw TCP/TLS connection that never sends the SRP1 header (e.g. `openssl s_client` or `nc`, connect and stay silent) | Agent-Probe (manual, human-executed) | Items 22-23 (2d handshake/accept hardening): the bad peer is timed out via `soTimeout` instead of wedging `handleClient` forever, and the accept loop `continue`s and keeps serving new connections afterward — this behavior had no verification scenario in the original plan and would otherwise be unproven by any gate |
| `just test-all` → `cd android-client && ./gradlew test` | Fully-Automated (compile-check only — NOT a behavior test; there is no `src/test/` or `src/androidTest/` in `android-client`, and this plan explicitly does not add one) | Compile-time regression guard only — proves the Kotlin changes build, proves nothing about the watchdog/teardown/notification behavior itself |
| `just lint-all` → `./gradlew lint` (android-client) | Fully-Automated | Code style / lint regression guard on Phase 2 changes |
| Manual: normal end-to-end session (connect, stream >30s, graceful disconnect) | Agent-Probe (manual, human-executed) | Do-Not-Regress #1: graceful disconnect and normal teardown still clean after the `AtomicReference` refactor |
| Manual: reconnect immediately after a stall-triggered disconnect | Agent-Probe (manual, human-executed) | Do-Not-Regress #3: no double-start of the capture job or watchdog after a watchdog-triggered teardown |
| Manual: Ctrl+C the PC client while idle and while mid-stream | Agent-Probe (manual, human-executed) | Do-Not-Regress #4: PC client still exits cleanly after `app_state.rs` was touched by Phase 1 — this check had no explicit gate in the original plan and must not be assumed covered by the other manual scenarios |

**Known gap (explicit, not silently dropped):** Phase 2 has zero automated behavioral test
coverage — `android-client` has no `src/test/` or `src/androidTest/` directory at all, and
building that infrastructure is explicitly out of scope for this plan (scope-creep guard from the
task brief). All Phase 2 behavioral proof is Agent-Probe (manual). This gap is accepted as a
known-gap for THIS plan, not resolved — a future "Android test infra" plan is the correct place to
close it, and is noted below as a Test Infra Improvement.

## Test Infra Improvement Notes

- `android-client` has no `src/test/` (unit) or `src/androidTest/` (instrumented) directory.
  `./gradlew test` today is a compile check only, not a behavior check — this was true before
  this plan and remains true after. A future plan should stand up at minimum a unit-test harness
  around `AudioCaptureService`'s pure logic (e.g. state-transition logic, watchdog threshold
  math) using a mockable `Socket`/`AudioRecord` seam, so watchdog and teardown-race behavior can
  get real automated coverage instead of only manual verification. Out of scope here.
- `pc-client/src/protocol.rs` remains the repo's only Rust unit-tested module; `app_state.rs`
  (where the VU-meter-zero and status-text changes land) has no unit tests and none are added by
  this plan — those two small UI-property changes are verified only by the manual scenarios above.
  This mirrors the existing, pre-plan gap noted in `all-context.md`'s Open Questions.

## Dependencies, Risks, Integration Notes

- No new external dependencies on either side (no socket2, no new crates, no new Gradle deps).
- Phase 1 and Phase 2 have no build or runtime dependency on each other (see Blast Radius) — can
  be executed and verified in either order, or in parallel.
- Risk: the watchdog threshold (item 21) is a judgment call with no hard measured number in the
  task brief — pick a concrete value in the low seconds during EXECUTE, document the reasoning
  inline, and treat it as tunable-but-hardcoded (no config surface).
- Risk: `AtomicReference` refactor (2a) touches 3+ call sites in one file — highest per-file blast
  radius in this plan. Do it first (as ordered) so 2b/2c build and test against the fixed
  structure, not the racy one.
- Integration note: the PC-side 5s header timeout (Phase 1) and the Android-side watchdog
  threshold (Phase 2, item 21) are deliberately NOT coupled to a shared constant or config value —
  they are independent per-side UX/liveness thresholds for different problems (PC: "when does a
  frozen meter look broken"; Android: "when do we consider the socket dead enough to force-close
  it"). Do not introduce a shared timeout constant between the two languages.

## Resume and Execution Handoff

1. **Selected plan file path:** `process/features/connection-resilience/active/connection-resilience_21-08-26/connection-resilience_PLAN_21-08-26.md`
2. **Last completed phase or step:** PLAN — plan written, not yet validated or executed.
3. **Validate-contract status:** pending (placeholder below — vc-validate-agent writes this section before EXECUTE).
4. **Supporting context files loaded:** `process/context/all-context.md`, `process/context/planning/all-planning.md`, `process/context/tests/all-tests.md`, `Justfile`, `pc-client/src/protocol.rs` (read in full for existing test harness pattern), `pc-client/src/app_state.rs` (lines ~121-500 scanned for retry loop / stream loop / status_text sites), `pc-client/ui/main.slint` (property declarations), GitHub issues #1-#4 research (pre-loaded into the locked design above — not re-fetched by this plan-agent session).
5. **Next step for a fresh agent picking up mid-execution:** Run VALIDATE on this plan file (`ENTER VALIDATE MODE`). Since Phases 1 and 2 are parallel-safe with zero file overlap, EXECUTE may run them via parallel subagents or sequentially at the executing agent's discretion — but 2a → 2b → 2c → 2d must stay strictly ordered within Phase 2 regardless of overall execution strategy.

## Acceptance Criteria

1. `cargo test test_header_read_timeout` passes and demonstrates the header read no longer blocks forever.
2. `just test-all` (both `cargo test` and `./gradlew test`) passes with no regressions.
3. `just lint-all` passes on both `pc-client` and `android-client`.
4. Manual scenario "kill PC client mid-stream": VU meter drops to zero and status text updates to a reconnecting message within ~5s.
5. Manual scenario "cut Wi-Fi mid-stream": PC UI reflects stall within ~5s; Android watchdog force-closes the stalled socket within the documented threshold.
6. Manual scenario "tap notification body mid-stream": connection stops immediately with clean teardown (no crash, no double-release).
7. Manual scenario "silent peer never sends SRP1": connection is timed out via `soTimeout` (not wedged indefinitely) and the server continues accepting subsequent connections afterward.
8. Do-Not-Regress checks 1-4 (graceful disconnect, both audio backends silence-on-underrun, no double-start on reconnect, Ctrl+C shutdown) all still pass.

## Phase Completion Rules

- Phase 1 (PC) is CODE DONE when checklist items 1-6 are complete and `cargo test` + `cargo clippy` are green.
- Phase 2 (Android) is CODE DONE when checklist items 7-26 are complete, in the required 2a -> 2b -> 2c -> 2d order, and `./gradlew test`/`./gradlew lint` are green (compile-check only, per the documented known-gap).
- Neither phase is VERIFIED until its manual Agent-Probe scenarios in Verification Evidence have been run and observed to match the expected outcome — code-complete is not the same as verified.
- The plan as a whole is Ready for UPDATE PROCESS archival only after both phases are VERIFIED and the Do-Not-Regress checks have been manually re-confirmed.

## Validate Contract

Status: PASS
Date: 21-08-26
date: 2026-08-21
generated-by: outer-pvl

Parallel strategy: parallel-subagents
Rationale: 7-signal score 2/7 (S6 high-risk class present: Android capture loop + TLS/SRP adjacency;
S7 5+ files in blast radius, since MainActivity.kt is a mandatory touch per corrected item 26, not
conditional). MEDIUM tier. Phase 1 (pc-client) and Phase 2 (android-client) are declared
zero-file-overlap, no-build-dependency, parallel-safe in Blast Radius — 2 independent directions
with no mid-execution coordination needed, so 2 fire-and-forget vc-execute-agent spawns (one per
phase) fit better than an agent team. Within Phase 2, items 2a -> 2b -> 2c -> 2d must stay
sequential inside that single agent's own execution (this is an ordering constraint inside one
agent's work, not a cross-agent coordination need).

Test gates (C3 5-column table):

| criterion id | behavior | strategy | proving test | gap-resolution |
|---|---|---|---|---|
| P1-header-timeout | PC: frame-header read no longer blocks forever, surfaces as Err | Fully-Automated | `cd pc-client && cargo test test_header_read_timeout` | A |
| P1-regression | PC: existing frame-parse/resync/overflow/degraded/gap-silence behavior unchanged | Fully-Automated | `cd pc-client && cargo test` | A |
| P1-lint | PC: style/lint regression guard | Fully-Automated | `cd pc-client && cargo fmt --check && cargo clippy -- -D warnings` | A |
| P1-vu-meter-zero | PC: VU meter drops to 0 and status text shows reconnecting message within ~5s of a kill | Agent-Probe | Manual: kill PC client (Ctrl+C or `kill -9` phone-side app) mid-stream | A |
| P1-wifi-stall | PC: UI reflects stall within ~5s; Android watchdog force-closes stalled socket within threshold | Agent-Probe | Manual: cut Wi-Fi mid-stream (disable on phone or PC host) | A |
| P2-notification-stop | Android: tapping notification stops connection immediately, clean teardown | Agent-Probe | Manual: tap notification body mid-stream | A |
| P2-android-compile | Android: Kotlin changes build (compile-check ONLY — no behavior proven) | Fully-Automated | `cd android-client && ./gradlew test` | A (compile-check scope only) |
| P2-android-lint | Android: style/lint regression guard | Fully-Automated | `cd android-client && ./gradlew lint` | A |
| P2-graceful-disconnect | Android: normal e2e session, graceful disconnect still clean after `AtomicReference` refactor | Agent-Probe | Manual: connect, stream >30s, graceful disconnect | A |
| P2-no-double-start | Android: no double-start of capture job/watchdog after watchdog-triggered teardown | Agent-Probe | Manual: reconnect immediately after a stall-triggered disconnect | A |
| P2-handshake-hardening | Android: silent peer (never sends SRP1) is timed out via `soTimeout`, server keeps accepting afterward | Agent-Probe | Manual: open raw TCP/TLS connection, send nothing, confirm timeout + server still serves next client (gate added by this validate pass — item 22-23 had no scenario in the original plan) | B |
| P1-ctrlc-shutdown | PC: Ctrl+C shutdown still exits cleanly after `app_state.rs` touched | Agent-Probe | Manual: Ctrl+C PC client while idle and mid-stream (gate added by this validate pass — Do-Not-Regress #4 had no scenario in the original plan) | B |
| P2-audio-backend-untouched | Do-Not-Regress: both audio backends' silence-on-underrun unaffected | N/A | No gate — structurally guaranteed: Phase 1 never touches `pc-client/src/audio/` at all (confirmed: no changes to `linux.rs`/`windows.rs` in this plan's Touchpoints) | A (proven by scope, not by running a test) |
| P2-android-behavior-automated | Android: teardown/watchdog/notification/handshake behavior proven by an automated (non-manual) test | Known-Gap | — no `src/test/`/`src/androidTest/` exists in `android-client`; building that infra is explicitly out of scope (locked INNOVATE decision, see Scope section) | D — backlog stub: `Test Infra Improvement Notes` section already in this plan names the follow-up ("Android test infra" plan, mockable `Socket`/`AudioRecord` seam) |

gap-resolution legend: A — proven now. B — fixed in this plan (gate added by this validate pass,
folded into Verification Evidence + Acceptance Criteria). C — deferred to a named later phase/plan.
D — backlog test-building stub (named residual; keep-active; continue).

C-4 reconciliation: every proven row above uses Fully-Automated or Agent-Probe (the only two of the
3 proving strategies this plan needs; Hybrid does not apply — no precondition-gated infra like a
running container/DB exists here). The one Known-Gap row (P2-android-behavior-automated) is a named
residual with written justification and an existing backlog stub — it is not the reason any
Android behavior passes; every Android behavior has an Agent-Probe row that proves it independently.

Legacy line form:
- PC header-timeout + regression: Fully-automated: `cd pc-client && cargo test`
- PC lint: Fully-automated: `cd pc-client && cargo fmt --check && cargo clippy -- -D warnings`
- Android compile-check: Fully-automated: `cd android-client && ./gradlew test` (compile-check only, not a behavior test)
- Android lint: Fully-automated: `cd android-client && ./gradlew lint`
- All Android behavior (teardown, notification, watchdog, handshake-hardening) + PC UI-property behavior (VU meter/status text): agent-probe: manual scenarios listed in Verification Evidence
- Android automated behavioral coverage: known-gap: documented as pre-existing, out-of-scope infra gap; see Test Infra Improvement Notes

Failing stub (P1-header-timeout, the one new Fully-Automated behavior added by this plan):
```
Failing stub:
test("should time out and return Err when the frame-header byte never arrives", () => {
  throw new Error("NOT IMPLEMENTED — TDD stub: read_next_frame magic-byte scan wrapped in a 5s tokio::time::timeout; Builder::new().wait(Duration::from_secs(10)).build() + #[tokio::test(start_paused = true)] per corrected item 5")
})
```
(Rust-native equivalent: the actual deliverable is the `#[tokio::test(start_paused = true)] async fn test_header_read_timeout()` specified verbatim in corrected checklist item 5 — this JS-shaped stub exists only to satisfy the validate-contract's stub format convention.)

Dimension findings:
- Infra fit: PASS — no container/infra/port surfaces exist in this repo; build orchestration is
  `cargo`/`gradlew` via `Justfile`, both confirmed present and correctly cited by the plan.
- Test coverage: PASS — Known-Gap (Android automated behavioral tests) is a locked INNOVATE-stage
  scope decision, not a plan gap; every developed behavior has an Agent-Probe or Fully-Automated
  proving row (none rest on Known-Gap alone); 2 coverage holes found during V2 (Ctrl+C shutdown,
  handshake-hardening) were closed by adding Agent-Probe rows directly to Verification Evidence +
  Acceptance Criteria.
- Breaking changes: PASS — no `'MC'` wire format, IOCTL contract, or external API change; the one
  additive internal surface (`ConnectionState` enum value + `ACTION_STOP` PendingIntent action) is
  correctly scoped as internal-only per Public Contracts section; confirmed both `MainActivity.kt`
  `when` sites are a **hard compile error**, not a soft fallthrough, if the enum grows — corrected
  in item 26.
- Security surface: PASS — no crypto/SRP/TLS logic changes; the `soTimeout` wrapper (item 22,
  corrected to cover the TLS handshake read too, not just the SRP reads) is a liveness guard around
  existing auth code, not a modification to it; confirmed `SocketTimeoutException` mid-handshake
  safely funnels into the existing general `catch`/`finally` teardown path — no new unhandled-state
  risk introduced.
- Section: Phase 1 (PC/Rust) feasibility: PASS — all touchpoint file:line references verified
  accurate against `protocol.rs`/`app_state.rs`/`main.slint`; `tokio-test = "0.4.5"` confirmed a real
  dev-dependency; `tokio = { features = ["full"] }` confirmed to include `test-util`
  (`start_paused` support); read the actual `tokio-test` 0.4.5 crate source
  (`~/.cargo/registry/src/.../tokio-test-0.4.5/src/io.rs`) to confirm `Builder::wait()` uses
  `tokio::time::sleep_until` internally (paused-clock-aware) and that a bare `Builder::new().build()`
  with no actions produces immediate EOF, not a hang — corrected item 5 accordingly. Highest-risk
  edit: the new timeout wrap in `read_next_frame`'s magic-byte scan loop — low risk, mirrors an
  existing pattern 30 lines below it in the same function.
- Section: Phase 2 (Android/Kotlin) feasibility: CONCERN (resolved in-plan) — 2 confirmed defects
  found (watchdog launched at the wrong scope/:117 in the original text; `audioRecord` remaining-site
  parenthetical pointed at the wrong line) plus 1 confirmed defect surfaced mid-session by the
  coordinator (same `audioRecord` finding, independently verified against a full grep of all 8
  occurrences) — all 3 corrected directly in the plan file. 4 additional gaps found during mechanical
  feasibility review, also corrected in-plan: (a) `ACTION_STOP` ordering/early-return was unstated —
  now explicit; (b) `soTimeout` scope excluded the TLS handshake read itself — now covers it; (c) the
  `ERROR`-state fix in item 24 would have been immediately overwritten by the finally block's
  unconditional CONNECTING reset — now guarded; (d) watchdog threshold range (3-5s) narrowed to a
  concrete "pick 5s, not 3s" recommendation with rationale (existing 24kHz-degradation safety valve
  absorbs ordinary jitter first). Highest-risk edit: the watchdog's `socket.close()` call from a
  second thread while `handleClient`'s loop may be blocked inside `recorder.read()` or
  `sendAudioPacket` — mitigated by keeping the fail-fast semantics on the local `recorder` reference
  (item 10) and by scoping+cancelling the watchdog per-connection (items 19-20), so a released
  `AudioRecord`/closed `Socket` deterministically throws into the existing `catch`/`finally`, not a
  new failure mode.

Open gaps:
- Android automated behavioral test coverage does not exist and is out of scope for this plan
  (locked INNOVATE decision) — tracked via the existing "Test Infra Improvement Notes" section, not
  a new backlog artifact.
- Minor line-drift, informational only, not corrected (immaterial to execution): checklist item 21's
  comment-placement citation ":393-405" for the existing 50ms-degradation block is slightly off —
  the actual `if (duration > 50) { ... }` block spans :392-410 in the current file. Anchor is close
  enough that an execute-agent reading the surrounding code will not be misled; no correction applied.

What this coverage does NOT prove:
- `cargo test test_header_read_timeout` proves the header-read path times out and returns `Err` — it
  does NOT prove the PC UI actually reflects that error (VU meter zeroing, status text change) —
  that is proven only by the Agent-Probe "kill PC client" / "cut Wi-Fi" scenarios, which are manual
  and not run by CI.
- `cd pc-client && cargo test` (full suite) proves `protocol.rs` framing/resync/overflow/degraded/gap
  logic is unchanged — it does NOT touch `app_state.rs` (VU-meter-zero, status-text, reconnect logic
  all live there) or `audio/linux.rs`/`audio/windows.rs` at all — those remain untested by any
  automated gate in this repo, before and after this plan.
- `./gradlew test` (android-client) is a **compile check only** — it proves the Kotlin changes build
  and the `ConnectionState` `when` exhaustiveness holds — it proves NOTHING about whether the
  `AtomicReference` teardown fix, the watchdog, the notification tap-to-stop, or the handshake
  hardening actually behave correctly at runtime. All runtime behavior proof for Phase 2 is
  Agent-Probe (manual, human-executed) — a real person on real hardware, not CI.
- `./gradlew lint` / `cargo clippy` prove code-style regressions only, not behavior.
- No gate in this plan (automated or manual) re-verifies the Windows kernel driver or the audio
  backends — both are confirmed untouched by this plan's Touchpoints, so this is a scope
  boundary, not a coverage hole.

Gate: PASS


## Autonomous Goal Block

```
SESSION GOAL: Close Border-Tech issues #1-#3 — PC-side stall detection, Android teardown race +
notification tap-to-stop, and watchdog/handshake hardening for the phone-mic connection.
Charter + umbrella plan: N/A — single plan (not a phase program; Phase 1/PC and Phase 2/Android are
parallel-safe sub-phases of this one plan, not separate umbrella-tracked phases).
Autonomy: standard /goal autonomous execution — CONDITIONAL findings apply-and-proceed, BLOCKED
items go to backlog and continue; irreversible/outward-facing actions without explicit contract
instruction are a hard stop. No live user hardware verification may be claimed as done by an agent.
Hard stop conditions / safety constraints:
- Do not touch pc-client/src/audio/ (linux.rs, windows.rs) — confirmed untouched, must stay that way.
- Do not modify SRP/TLS handshake crypto logic itself — only wrap it with soTimeout/timeout.
- Do not add socket2, TCP keepalive tuning, an app-level heartbeat frame, or change the 'MC' wire
  format — all explicitly rejected in INNOVATE and out of scope.
- Do not add a Stop button or wire the notification body to open MainActivity — rejected by design.
- Do not build Android test infrastructure (src/test/, src/androidTest/) — explicitly out of scope.
- All 7 Agent-Probe manual scenarios in Verification Evidence require a human on real hardware
  (phone + PC, and for P2-handshake-hardening a raw TCP client) — an agent cannot self-certify these;
  EXECUTE may implement and pass the automated gates, but manual scenarios stay open until a human
  runs them.
Next phase: EXECUTE: process/features/connection-resilience/active/connection-resilience_21-08-26/connection-resilience_PLAN_21-08-26.md
(Phase 1 pc-client and Phase 2 android-client may run as 2 parallel vc-execute-agent spawns —
zero file overlap, no build dependency; within Phase 2, items 2a->2b->2c->2d must stay sequential.)
Validate contract: inline in plan (## Validate Contract section, this file) — Gate: PASS.
Execute start: fully-automated: `cd pc-client && cargo test && cargo fmt --check && cargo clippy -- -D warnings`
| `cd android-client && ./gradlew test && ./gradlew lint` (compile-check only for Android) |
probe scenarios: kill-PC-client, cut-Wi-Fi, tap-notification, normal-e2e-session,
reconnect-after-stall, silent-peer-no-SRP1, Ctrl+C-shutdown (7 total, all human-executed) |
high-risk pack: no (no auth/billing/schema/migration/public-API/deploy logic changed — only a
soTimeout liveness wrapper around existing, unmodified crypto; TLS/SRP adjacency noted but not
touched in substance, per Blast Radius).
```
