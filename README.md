# Project-M: High-Fidelity Wireless Audio Link

Project-M is a high-performance, low-latency wireless microphone system that turns your Android phone into a virtual PC microphone. It replaces the proprietary WO Mic connection with a custom, lossless, raw PCM streaming protocol over TCP. 

This repository is organized as a monorepo containing:
1. **`pc-client/`**: A native Linux receiver (Rust & Slint UI) that streams incoming audio directly into a PipeWire virtual microphone.
2. **`android-client/`**: A modern Android app (Kotlin & Jetpack Compose) that captures microphone audio and streams it to the PC.

---

## Technical Specifications
* **Protocol**: Custom TCP framed with Magic bytes `['M', 'C']` and 2-byte big-endian payload length.
* **Audio Format**: Raw 16-bit Signed PCM, Little-Endian (`s16le`), Mono.
* **Sample Rate**: 48,000 Hz.
* **Jitter Buffer**: Lock-free ring buffer (via `ringbuf` in Rust) for ultra-low latency playback.
* **Compatibility**: Optimized for Linux systems using PipeWire.

---

## Directory Structure
```
project-m/
├── Cargo.toml                  # Cargo Workspace configuration
├── pc-client/                  # Rust Linux client (Slint UI)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs             # Receiver entry & UI runner
│   │   ├── audio.rs            # PipeWire playback loop
│   │   ├── protocol.rs         # TCP server framing receiver
│   │   └── app_state.rs        # UI state & ADB port reverse setup
│   └── ui/                     # Slint files
└── android-client/             # Android app (Kotlin & Jetpack Compose)
    ├── settings.gradle.kts
    └── app/                    # AudioRecord, TCP client, Foreground Service
```

---

## How to Build & Run

### 1. Linux PC Client (`pc-client/`)
Ensure you have the following system dependencies installed (e.g. on Arch/CachyOS):
```bash
sudo pacman -S base-devel pkgconf pipewire libpipewire android-tools
```

Build and run:
```bash
cargo build --release
./target/release/project-m
```

### 2. Android App (`android-client/`)
1. Open the `android-client/` folder in **Android Studio** or **IntelliJ IDEA**.
2. Build the project (Gradle will automatically bootstrap and configure the project).
3. Connect your Android phone with **USB Debugging** enabled.
4. Run the app on your phone.

---

## Usage Instructions

### Connection via USB (Recommended for lowest latency)
1. In the PC Client UI, toggle the mode to **USB (ADB)** and click **Start Listening**.
2. In the Android App UI, enter `127.0.0.1` as the Receiver IP and click **START**.
3. *Note: The PC client automatically configures ADB reverse port forwarding (`adb reverse tcp:47999 tcp:47999`), so any traffic from the phone's localhost goes directly to the PC.*

### Connection via Wi-Fi
1. Ensure both the phone and PC are on the same local network.
2. In the PC Client UI, toggle the mode to **Wi-Fi**. It will display your PC's local IP address (e.g., `192.168.1.120`).
3. Click **Start Listening** on the PC.
4. In the Android App UI, enter the PC's IP address (e.g., `192.168.1.120`) and click **START**.

### System Configuration
Open your Linux system volume settings (or `pavucontrol`) and select **"Project-M-Virtual-Mic"** as your input device for Discord, OBS, Zoom, or any other application.

---

## License
MIT
