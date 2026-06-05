# Project-M — Remaining Findings

**Final Re-verification:** 2026-06-05

---

## NOT RESOLVED

| ID | Finding | File | Why It's Still Open |
|---|---|---|---|
| **C-03** | No authentication/integrity on the stream | `pc-client/src/protocol.rs` `android-client/.../AudioCaptureService.kt` | Both sides rely solely on the 2-byte `MC` magic header. PC's `DummyVerifier` accepts any TLS certificate — encryption without authentication. Needs a handshake with mutual auth + session key. |
| **M-02** | Raw FFI dangling pointer | `pc-client/src/audio.rs` | `unsafe { pw_sys::pw_main_loop_quit(mainloop_ptr); }` remains unprotected. No safety comment. |
| **M-07** | Kotlin 2.0.0 | `android-client/gradle/libs.versions.toml:3` | Version catalog pins Kotlin 2.0.0. Latest stable is 2.1.x. |
| **L-03** | FFI safety comment missing (regression) | `pc-client/src/audio.rs` | `unsafe` block has no `// SAFETY:` comment explaining preconditions. |

## PERFORMANCE

| ID | Finding | File | Notes |
|---|---|---|---|
| **P-01** | Find a way to decrease delay | `pc-client/src/audio.rs` `android-client/.../AudioCaptureService.kt` | Current pipeline: audio capture (Android) → TLS send → network → TLS receive → ring buffer → jitter buffer → PipeWire. Each stage adds latency. Investigate: smaller frame size (current 960 bytes = 20ms), jitter buffer threshold tuning, direct buffer passthrough, real-time thread priorities. |

## PARTIALLY RESOLVED

| ID | Finding | File | Done | Still Missing |
|---|---|---|---|---|
| **L-01** | 8.8.8.8 dependency for local IP | `pc-client/src/main.rs` | Added fallback chain: `8.8.8.8` → `1.1.1.1` → `224.0.0.1` | Still makes external network contact on startup. Could query local addresses directly. |

## WITHDRAWN (by design)

| ID | Finding | Severity | Reason |
|---|---|---|---|
| C-01 | Android TCP server on all interfaces | CRITICAL | Intentional — matches WO Mic architecture. Android is the server; PC connects to it. |
| H-01 | ADB command injection | HIGH | Intentional — user chooses port; `u16` parse prevents injection. `which adb` resolves PATH vector. |
