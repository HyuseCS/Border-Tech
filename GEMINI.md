# Project-M: Border Tech Ecosystem

## Project Overview
**Project-M (Border Tech Ecosystem)** is a suite of cross-platform applications designed to integrate Android devices into professional desktop workflows. The core functionality provides a high-fidelity, ultra-low latency wireless microphone pipeline by streaming raw 48kHz PCM audio over TCP/TLS from an Android device directly to a PC.

The repository is structured into three main components:
1. **Lampyris (`pc-client/`)**: The central desktop hub built in Rust with a Slint UI. It acts as a secure TLS client, manages ADB tunneling, and streams audio directly into Linux PipeWire virtual sources.
2. **Sonus (`android-client/`)**: A high-performance Android application built with Kotlin and Jetpack Compose. It acts as a TCP/TLS Server (mimicking pro hardware behavior) capturing and sending audio.
3. **Windows Virtual Audio Driver (`windows-driver/`)**: A C++ kernel-mode virtual audio driver to eventually support the ecosystem on Windows by securely receiving audio payloads via IOCTL.

## Building and Running
The project uses `just` as a top-level command runner. 

### Global Commands (via `Justfile`)
*   `just build-all`: Builds both the PC client (`cargo build --release`) and the Android client (`./gradlew assembleRelease`).
*   `just test-all`: Runs test suites for PC (`cargo test`) and Android (`./gradlew test`).
*   `just lint-all`: Runs linters/formatters (`cargo fmt`, `cargo clippy`, and `./gradlew lint`).
*   `just audit`: Runs dependency security checks (`cargo deny check` and `./gradlew dependencyCheckAnalyze`).
*   `just bench-latency`: Runs latency benchmarks for the PC client (`cargo bench`).

### Component-Specific
*   **Linux PC Client**: Navigate to `pc-client/` and run `cargo run --release`. Ensure dependencies like `pipewire`, `pkgconf`, and `android-tools` are installed.
*   **Android App**: Navigate to `android-client/` and use `./gradlew installDebug` or `./gradlew assembleRelease`.
*   **Windows Driver**: Must be compiled on Windows using MSVC and the Windows Driver Kit (WDK) via `windows-driver/lampyris-mic.sln`. Installation requires test-signing mode enabled.

## Development Conventions

### Security & Architecture
*   **Protocol Framework**: The architecture is "flipped" - the Android device is the TCP/TLS server, and the PC is the client.
*   **Data Framing**: Uses a custom `['M', 'C']` Magic Marker + 2-byte Big-Endian Length + PCM Payload.
*   **Security**: Mandatory end-to-end TLS 1.3 / 1.2 encryption using dynamic, on-device generated X.509 certificates.
*   **Windows Security Isolation**: The Windows driver uses an authenticated session token registry handshake (`IOCTL_LAMPYRIS_AUTHENTICATE`) to secure user-space interaction.

### Design System (The "Neon Audio Deck")
The ecosystem strictly adheres to a cohesive "Tactical Neon Hi-Fi" design system defined in `DESIGN.md`. Ensure these principles are maintained:
*   **Color Palette**: Use a dark space-black background (`#0A0A0E`, `#14141B`) with high-contrast neon accents.
*   **15% Accent Rule**: Neon Cyan (`#00FFCC`) for active states and Neon Pink (`#FF3366`) for errors/stopped states. Accents must occupy ≤15% of the screen.
*   **Elevation**: Use tonal layering and thin translucent colored borders (`1px solid #00FFCC4D` for active) rather than drop shadows.
*   **Typography**: Monospace (Courier New) for Display/Brand titles and Sans-serif (Inter/Arial) for body and labels.
*   **Anti-patterns**: Avoid generic Material Design defaults, light generic themes, and standard system borders. Do not use gradient text.
