# Windows IOCTL Mismatch Resolution

## The Issue
The Windows kernel driver enforces a strict `InputBufferLength` requirement (`sizeof(LAMPYRIS_AUDIO_PAYLOAD)` == 4804 bytes) for the `IOCTL_LAMPYRIS_PUSH_AUDIO` operation. The Rust PC client historically passed a dynamically calculated size (`size_of::<u32>() + audio_payload.length`), which caused the driver to return `STATUS_INVALID_PARAMETER` when pushing partially filled audio buffers.

## The Fix
The Rust PC client must be updated to always pass the absolute constant struct size (`std::mem::size_of::<LampyrisAudioPayload>() as u32`) in the `DeviceIoControl` call. The kernel driver safely extracts the actual valid byte count using the internal `Length` struct field, making this constant-buffer-size approach extremely safe and resolving the mismatch without requiring driver recompilation.
