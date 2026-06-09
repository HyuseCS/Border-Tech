# Autonomous Goal: Complete Phase 3 (Cross-Platform OS Abstraction)

## Objective
Your goal is to fully execute **Phase 3** of `execution_roadmap.md` within the `pc-client/` Rust project. Do not stop working or wait for user input until the Rust code successfully compiles, the Windows driver bridge is fully implemented, and it is ready for end-to-end user testing. 

## Task List
You are to perform the following steps sequentially in the `pc-client` Rust workspace:

### 1. Refactor Audio Abstraction Layer (Phase 3.1)
- Examine the existing `pc-client/src/audio/` module (or `audio.rs` if it's a single file).
- Abstract the current audio sink logic into a generic `AudioBackend` trait.
- Move the existing Linux PipeWire logic into a new `pc-client/src/audio/linux.rs` file.

### 2. Implement Windows Backend (Phase 3.2)
- Create `pc-client/src/audio/windows.rs`.
- Ensure the `windows` (or `windows-sys` / `winapi`) crate is included in `Cargo.toml` with the necessary features (`Win32_Storage_FileSystem`, `Win32_System_IO`, `Win32_Foundation`, `Win32_System_Registry`).
- **Bridge Logic Implementation:**
  - Implement a mechanism to read the 32-byte `SessionToken` from the Windows Registry (`HKLM\SOFTWARE\Lampyris`).
  - Open a handle to `\\.\LampyrisMic` (or `\\.\DosDevices\LampyrisMic`) using `CreateFileW`.
  - Send the token via `DeviceIoControl` using `IOCTL_LAMPYRIS_AUTHENTICATE`.
  - Implement the audio streaming loop: accept PCM data and push it via `DeviceIoControl` using `IOCTL_LAMPYRIS_PUSH_AUDIO` (ensuring payloads never exceed 4,800 bytes).

### 3. Integration & Testing
- Integrate the OS-specific audio backends into the main client loop using conditional compilation (`#[cfg(target_os = "windows")]` and `#[cfg(target_os = "linux")]`).
- Run `cargo check` and `cargo fmt` to verify your code. Fix any compilation errors.
- Ensure the client compiles flawlessly on the Windows host.

## Completion Criteria
Do not pause for user feedback until:
1. `pc-client` compiles completely without errors on Windows.
2. The `windows.rs` implementation correctly maps to the IOCTL structures defined in the `windows-driver` C++ project.
3. You are ready to instruct the user to run the PC client and test the stream from their Android device.
