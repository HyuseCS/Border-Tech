# Border Tech: Phone to PC Ecosystem

This repository houses the **Border Tech Ecosystem**, a suite of cross-platform tools designed to seamlessly integrate Android devices into professional desktop workflows.

The ecosystem currently consists of two core applications:
1. **Lampyris (PC Headquarters/Hub)**: The central desktop hub for all current and future feature apps. Built in Rust with a sleek Slint UI, it currently manages secure TLS connections, ADB tunneling, and PipeWire virtual audio integration for Linux.
2. **Sonus (Android Audio Server)**: A high-performance, low-latency wireless microphone app with a "Cinema Mobile" aesthetic that transforms your Android device into a professional-grade virtual PC microphone.

## Key Features
*   **Lampyris Headquarters**: Designed from the ground up to be the central hub for all future mobile-to-PC feature apps.
*   **Secure by Design**: Mandatory end-to-end TLS 1.3/1.2 encryption using dynamic, on-device generated X.509 certificates.
*   **Flipped Architecture**: Android acts as a **TCP/TLS Server** (mimicking pro hardware behavior), while the PC (Lampyris) acts as the secure client.
*   **Ultra-Low Latency**: Optimized for 48kHz, 16-bit PCM streaming directly into Linux **PipeWire** virtual sources.
*   **Automated Tunneling**: Built-in `adb forward` integration in Lampyris for seamless one-click USB connectivity.
*   **Modern UI Interfaces**: Glassmorphism and ambient visuals on the Android client (Sonus), with a clean, responsive desktop interface on the PC hub (Lampyris).

---

## Technical Specifications
*   **Audio Pipeline**: 48,000 Hz, 16-bit Signed PCM (s16le), Mono.
*   **Security Protocol**: TLS 1.3 / 1.2 (RSA-2048 / AES-GCM) with BouncyCastle provider.
*   **Protocol Frame**: `['M', 'C']` (Magic Marker) + `[Length]` (2B Big-Endian) + `[PCM Payload]`.
*   **Network Ports**: Default 47999 (Configurable).

---

## Repository Structure
```text
project-m/
├── pc-client/                  # Lampyris: Rust-based Receiver (Slint UI)
│   ├── src/
│   │   ├── main.rs             # CLI & App entry
│   │   ├── audio.rs            # PipeWire & Sink management
│   │   ├── protocol.rs         # 'MC' Framing & PCM processing
│   │   └── app_state.rs        # TLS Client, ADB Forwarding & UI Bridge
│   └── ui/                     # Slint UI definitions
└── android-client/             # Sonus: Android Client (Kotlin & Compose)
    └── app/src/main/java/...
        ├── MainActivity.kt      # Cinema UI & robust IP fetching
        └── AudioCaptureService.kt # TLS Server & PCM capture loop
```

---

## Getting Started

### 1. Linux PC Client (`pc-client/`)
Install system dependencies (Arch/CachyOS example):
```bash
sudo pacman -S base-devel pkgconf pipewire libpipewire android-tools
```

Build and run:
```bash
cd pc-client
cargo run --release
```

### 2. Android App (`android-client/`)
1. Connect your device with USB Debugging enabled.
2. Build and install the optimized release:
   ```bash
   cd android-client
   ./gradlew installDebug # For testing
   ./gradlew assembleRelease # For final Sonus-v1.0.apk
   ```

---

## Usage Instructions

### Connection via USB (Recommended)
1. Select **USB (ADB)** mode in the **Sonus** Android app and click **START**.
2. Launch the PC Client, select **USB (ADB)**, and click **Connect**.
3. *Note: The PC client automatically handles `adb forward` tunneling.*

### Connection via Wi-Fi
1. Ensure both devices are on the same Wi-Fi network.
2. In the **Sonus** Android app, select **WI-FI** mode. The device IP will appear automatically.
3. Click **START** on the phone, then enter that IP in the PC Client and click **Connect**.

### System Integration
Once connected, open your system sound settings (e.g., `pavucontrol`) and select **"Lampyris-Virtual-Mic"** as your input source.

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
