fix(pc-client): resolve IOCTL audio push buffer size mismatch and document auth error

- pc-client: Changed the dynamic buffer size parameter in the `DeviceIoControl` call for `IOCTL_LAMPYRIS_PUSH_AUDIO` to the constant `LampyrisAudioPayload` struct size (`std::mem::size_of::<LampyrisAudioPayload>() as u32`) to prevent the kernel driver from rejecting requests with STATUS_INVALID_PARAMETER.
- pc-client: Cleaned up compiler warnings by removing the unused `REG_VALUE_TYPE` registry import and removing unnecessary mutability on `auth_payload`.
- docs: Added `windows_auth_access_denied.md` containing root cause analysis and a resolution plan for the `Access is denied (0x80070005)` authentication handshake issue.
