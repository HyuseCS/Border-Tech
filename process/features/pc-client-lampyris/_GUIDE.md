# pc-client-lampyris

<!-- Part of Project-M -->

## Scope

Lampyris, the Rust desktop hub (`pc-client/`, crate `lampyris` v1.1.0). Covers the Slint UI, the CLI entry point, the TLS/SRP client that dials the Android phone, the `'MC'` frame decoder, the cross-platform audio abstraction, and the `adb forward` USB tunnelling. It is designed as the central hub for future phone-to-PC feature apps, so treat additions as hub features, not mic-only features.

Does NOT cover: the kernel driver itself (see `windows-driver`) or anything on the phone (see `android-client-sonus`). It DOES cover the user-mode side of the IOCTL bridge (`src/audio/windows.rs`).

## Key Source Files

- `pc-client/src/main.rs` — clap CLI (`--port`, `--debug`), tracing-to-file setup, Slint `MainWindow` wiring, `Arc<AppState>` creation, Ctrl+C shutdown
- `pc-client/src/app_state.rs` (524 loc, largest file) — TLS client, custom `ServerCertVerifier`, SRP pairing, `adb forward`, reconnect (`MAX_RETRIES = 5`), UI bridge, `convert_s16_to_f32`, `calculate_peak_amplitude`
- `pc-client/src/protocol.rs` — `ProtocolHandler`: `'MC'` magic scan, 7-byte header parse, sequence tracking. **The only unit-tested module in the repo.**
- `pc-client/src/audio/mod.rs` — the `AudioBackend` trait and the `cfg`-gated `DefaultAudioBackend` alias
- `pc-client/src/audio/linux.rs` — PipeWire virtual source sink
- `pc-client/src/audio/windows.rs` — `DeviceIoControl` bridge to the kernel driver
- `pc-client/ui/main.slint` — Slint UI definition (built by `slint-build`, pulled in via `slint::include_modules!()`)
- `pc-client/Cargo.toml` — note the two `[target.'cfg(...)']` dependency blocks
- `pc-client/.cargo/` — cross-compilation config (MinGW target for Windows)

## Related Context

- `process/context/all-context.md` — stack, wire contract, conventions
- `process/context/tests/all-tests.md` — `cargo test`, the `cfg`-gating pitfall, log-file location
- `process/features/windows-driver/_GUIDE.md` — the other half of the IOCTL contract

## Current Status

Status: stable

Phase 3 (cross-platform OS abstraction) is complete and verified end to end on a Windows VM. The producer side — TLS handshake, SRP auth, PCM into the kernel ring buffer — works. Active work is downstream in the driver, not here.

## Folder Contents

```
process/features/pc-client-lampyris/
  active/       -- in-progress plans for this feature (each task lives inside a {slug}_{date}/ task folder)
  completed/    -- archived completed plans
  backlog/      -- deferred/future plans
```

All artifacts (plans, specs, reports, references) colocate inside each `{slug}_{date}/` task folder. Do NOT create `reports/` or `references/` sibling dirs.
