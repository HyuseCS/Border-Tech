# Sonus: Professional TLS-Encrypted Wireless Audio Link

Sonus is a high-performance, low-latency wireless microphone system that transforms your Android device into a professional-grade virtual PC microphone. Unlike proprietary solutions, Sonus provides **end-to-end TLS 1.3 encryption** and uses a raw, lossless PCM streaming protocol for maximum audio fidelity.

## Key Features
*   **Secure by Design**: Mandatory TLS 1.3/1.2 encryption with on-device dynamic X.509 certificate generation.
*   **Flipped Architecture**: Android acts as the **Server** (mimicking pro hardware behavior), allowing the PC client to initiate secure connections.
*   **Ultra-Low Latency**: Optimized for Linux PipeWire with a lock-free jitter buffer.
*   **Cinema UI**: A modern "Cinema Mobile" aesthetic for the Android client with glassmorphism and ambient visuals.
*   **Dual-Mode Connectivity**: Seamless support for both **Wi-Fi** and **USB (ADB)** tunneling.

---

## Technical Specifications
*   **Audio Format**: Raw 16-bit Signed PCM, Little-Endian (`s16le`), Mono.
*   **Sample Rate**: 48,000 Hz.
*   **Encryption**: TLS 1.3 / TLS 1.2 (RSA-2048 / AES-GCM).
*   **Framing**: Custom 'MC' header (2B) + Payload Length (2B) + PCM Data.
*   **Backend**: PipeWire virtual source integration for Linux.

---

## Repository Structure
```
project-m/
├── pc-client/                  # Rust Receiver (Slint UI)
│   ├── src/
│   │   ├── main.rs             # Application entry
│   │   ├── audio.rs            # PipeWire & Sink management
│   │   ├── protocol.rs         # PCM Framing logic
│   │   └── app_state.rs        # TLS Client & ADB Forwarding logic
│   └── ui/                     # Slint UI definitions
└── android-client/             # Android App (Kotlin & Compose)
    └── app/src/main/java/...
        ├── MainActivity.kt      # Cinema UI & IP retrieval
        └── AudioCaptureService.kt # TLS Server & Audio capture loop
```

---

## How to Build & Run

### 1. Linux PC Client (`pc-client/`)
Ensure you have the following system dependencies installed (e.g., on Arch/CachyOS):
```bash
sudo pacman -S base-devel pkgconf pipewire libpipewire android-tools
```

Build and run:
```bash
cd pc-client
cargo run --release
```

### 2. Android App (`android-client/`)
1. Open the `android-client/` folder in **Android Studio**.
2. Connect your Android phone with **USB Debugging** enabled.
3. Build the **Release** APK:
   ```bash
   ./gradlew assembleRelease
   ```
4. Install the debug version for testing:
   ```bash
   ./gradlew installDebug
   ```

---

## Usage Instructions

### Connection via USB (Recommended)
1. Connect your phone via USB and ensure ADB is authorized.
2. In the **Sonus** Android app, select **USB (ADB)** mode and click **START**.
3. In the PC Client, select **USB (ADB)** and click **Connect**.
4. *Note: The PC client automatically runs `adb forward tcp:47999 tcp:47999` to tunnel the encrypted stream.*

### Connection via Wi-Fi
1. Ensure both devices are on the same local network.
2. In the **Sonus** Android app, select **WI-FI** mode. Note the IP address displayed on the screen.
3. Click **START** on the Android app.
4. In the PC Client, enter the phone's IP address and click **Connect**.

### System Setup
Once connected, open your Linux sound settings (or `pavucontrol`) and select **"Lampyris-Virtual-Mic"** (Sonus Virtual Source) as your default input device.

---

## License
MIT
