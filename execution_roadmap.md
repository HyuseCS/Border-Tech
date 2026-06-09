# Execution Roadmap: Project-M Custom Audio Streaming

## Objective
Execute the transition from proprietary audio streaming to a custom, low-latency, cross-platform audio pipeline. This roadmap prioritizes hardening existing Android and Linux infrastructure before developing the net-new Windows Kernel Driver and cross-platform abstractions.

---

## Phase 1: Security & Protocol Hardening
*Focus: Securing the existing PC and Android clients, upgrading the transport protocol, and establishing CI/CD.*

### 1.1 Security Hardening (P0 Blocker) - **[COMPLETED]**
*   **Certificate Pinning:** Replace the `DummyVerifier` in `pc-client/src/app_state.rs` with Trust On First Use (TOFU) ECDSA certificate pinning.
*   **Authentication:** Implement SRP (Secure Remote Password) using the 6-digit UI PIN.
    *   *Android:* Integrate BouncyCastle SRP.
    *   *Rust:* Integrate the `srp` crate with custom M1/M2 hash matching for compatibility.
    *   Implement brute-force lockout logic (3 attempts → 30s → 5m → reset).
*   **Keystore & Network Hygiene:**
    *   Replace the hardcoded Android keystore password with a runtime `SecureRandom` value.
    *   Bind the Android TLS server exclusively to `127.0.0.1` when operating in USB/ADB mode.
    *   Remove plaintext PINs from all log outputs.

### 1.2 Protocol Upgrades (v1 Framing) - **[COMPLETED]**
*   **Framing Implementation:** Update `protocol.rs` and the Android `AudioCaptureService` to implement the v1 framing:
    *   Magic bytes (`MC`), Version/Flags (1B), Sequence Number (2B BE), Payload Size (2B BE).
*   **Constraints & Resiliency:**
    *   Enforce a strict 4,800-byte maximum payload size limit in `protocol.rs`.
    *   Implement sequence gap detection and silence insertion.
*   **Degraded Mode:** Implement sender-driven dynamic sample rate scaling (48kHz to 24kHz fallback) based on TCP backpressure, with zero-dependency linear upsampling on the receiver.

### 1.3 Build & CI/CD Orchestration - **[COMPLETED (CI/CD Aborted)]**
*   **Monorepo Setup:** Create a top-level `Justfile` for unified commands (`build-all`, `test-all`, `lint-all`, `bench-latency`). **[Done]**
*   **GitHub Actions:** Define workflows for Rust and Android. **[Aborted - System does not require CI/CD]**
*   **Cleanup:** Remove legacy compiled binaries from git history using `git filter-branch` and update `.gitignore`. **[Done]**

---

## Phase 2: Windows Driver Prototype
*Focus: Tackling the highest-risk architectural component—the Windows Kernel Virtual Audio Driver.*

### 2.1 Driver Initialization - **[COMPLETED]**
*   **Scaffolding:** Fork the Microsoft `sysvad` sample into a new `windows-driver/` directory.
*   **Configuration:** Set up the Visual Studio Solution and WDK build configurations.

### 2.2 IOCTL Implementation & Security - **[COMPLETED]**
*   **Data Handling:** Implement `METHOD_BUFFERED` IOCTLs for receiving PCM data.
*   **Validation:** Strictly validate all incoming buffer sizes against the 4,800-byte limit.
*   **Access Control:**
    *   Apply restrictive DACLs to the device object (Interactive User + SYSTEM only).
    *   Implement load-time shared-secret generation and registry-based authentication to prevent rogue local injection.

### 2.3 Verification & Testing - **[ACTIVE]**
*   **Deployment:** Install and run the driver on a Test-Mode Windows machine.
*   **Static Analysis:** Integrate Driver Verifier and HLK static analysis into the `driver.yml` CI workflow.

---

## Phase 3: Cross-Platform OS Abstraction
*Focus: Refactoring the Rust PC client to support heterogeneous audio backends seamlessly.*

### 3.1 Audio Abstraction Layer
*   **Refactoring:** Abstract `pc-client/src/audio.rs` into a generic `AudioBackend` trait.
*   **Linux Implementation:** Move existing PipeWire logic into `pc-client/src/audio/linux.rs`.

### 3.2 Windows Integration
*   **Backend Implementation:** Create `pc-client/src/audio/windows.rs` utilizing the `windows-rs` crate.
*   **Bridge Logic:** Implement `DeviceIoControl` logic to negotiate the shared secret and stream the lock-free ring buffer data to the new Kernel Driver.

---

## Phase 4: System Polish & Release Readiness
*Focus: Ensuring rock-solid stability, verifiable low latency, and production-grade observability.*

### 4.1 Fuzzing & Error Handling
*   **Fuzzing:** Implement `cargo-fuzz` targets for the `protocol.rs` parser (targeting 0 panics).
*   **Error Propagation:** Standardize on `anyhow` and a custom `thiserror` `AppError` enum. Enforce `#![deny(clippy::unwrap_used)]`.

### 4.2 Observability & Diagnostics
*   **Structured Logging:** Implement `tracing` with JSON output and log rotation on the PC client. Use `Timber` on Android.
*   **Diagnostic Export:** Build the UI feature to generate a sanitized, local ZIP archive containing logs and latency histograms for user bug reports.

### 4.3 Benchmarking & Latency Verification
*   **CI Gates:** Implement the `just bench-latency` target and enforce a processing latency gate (`p99 < 2ms`) in GitHub Actions.
*   **Hardware Verification:** Conduct manual end-to-end loopback tests to confirm targets (USB ≤ 10ms, Wi-Fi ≤ 20ms).

### 4.4 Release Operations
*   **Distribution:** Automate binary generation and SHA-256 checksum creation.
*   **Publishing:** Configure workflows to publish artifacts via GitHub Releases exclusively.