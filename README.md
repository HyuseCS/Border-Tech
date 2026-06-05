# WO Mic Client for Linux (Native)

A high-performance, low-latency, native Linux client for the WO Mic mobile application. This replica is optimized for CachyOS and Arch-based systems, using PipeWire for modern audio integration.

## Features

- **Native Rust Core:** Built for performance and memory safety.
- **PipeWire Integration:** Creates a virtual microphone directly in the PipeWire graph.
- **Low Latency:** Uses a lock-free ring buffer (via `ringbuf`) for zero-allocation audio streaming.
- **Native UI:** A sleek, lightweight interface built with Slint.
- **USB & WiFi Support:** Automatic ADB port forwarding for low-latency USB connections.
- **Hardened Security:** Input validation and robust protocol resynchronization.
- **Graceful Lifecycle:** Automatic cleanup of PipeWire nodes and ADB rules on exit.

## Prerequisites

Ensure you have the following system dependencies installed:

```bash
sudo pacman -S base-devel pkgconf pipewire libpipewire android-tools opus
```

## Installation & Build

1. **Clone the repository.**
2. **Build the release version:**
   ```bash
   cargo build --release
   ```
3. **Run the client:**
   ```bash
   ./target/release/womic-linux
   ```

## Usage

1. **Open WO Mic on your Android phone.**
2. **Select Transport:** Choose "USB" or "WiFi" in both the phone app and this client.
3. **Connect:** Click "Connect" in the client UI.
4. **Configure System Audio:** Open your sound settings (or `pavucontrol`) and select **"WOMIC-Virtual-Mic"** as your input device.

## Security Note

- **USB Mode:** Highly recommended for lowest latency and maximum security (local only).
- **WiFi Mode:** Use with caution on public networks as the WO Mic protocol is unencrypted (inherent limitation of the proprietary protocol).

## Technical Architecture

- **`audio.rs`**: Manages the PipeWire stream and background main loop. Uses Atomic flags and SPA timers for safe cleanup.
- **`protocol.rs`**: Implements the WO Mic handshake and a byte-by-byte resync loop to handle stream misalignment.
- **`app_state.rs`**: Orchestrates connection tasks, cancellation tokens, and UI synchronization.
- **`decoder.rs`**: Provides high-performance PCM and Opus decoding.

## License

MIT
