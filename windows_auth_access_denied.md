# Windows Driver Authentication Failure: Access is Denied (0x80070005)

## The Issue
When running the PC client (`lampyris.exe`) in the VM, it fails during the authentication handshake with the following error:
```
Error: Failed to authenticate with LampyrisMic: Err(Error { code: HRESULT(0x80070005), message: "Access is denied." })
```

This error indicates that the `DeviceIoControl(..., IOCTL_LAMPYRIS_AUTHENTICATE, ...)` call was rejected by the Windows kernel/driver with `STATUS_ACCESS_DENIED` (`0xC0000022`), which gets translated to Win32 `ERROR_ACCESS_DENIED` (`5` / `0x80070005`).

---

## Potential Root Causes

### 1. Registry Key Creation Failures & Stale Tokens
* **The Mechanism:** The kernel-mode driver writes the session token using `RtlWriteRegistryValue` targeting `\Registry\Machine\Software\Lampyris`.
* **The Flaw:** In `windows-driver/lampyris-sysvad/lampyris_core.cpp`, the return value of `WriteTokenToRegistry()` is ignored:
  ```cpp
  // Inside lampyris_core.cpp:
  WriteTokenToRegistry(); // Discards NTSTATUS
  ```
  If the `HKLM\SOFTWARE\Lampyris` registry key does not exist on the target machine, `RtlWriteRegistryValue` fails with `STATUS_OBJECT_NAME_NOT_FOUND` (because it doesn't automatically create parent subkeys in the registry tree).
* **The Outcome:** The driver continues to load successfully with a newly generated random token in memory, but the registry is not updated. If a stale token from a previous driver run or manual installation remains in `HKLM\SOFTWARE\Lampyris`, the user-mode client reads that stale token, sends it to the driver, and is rejected with `STATUS_ACCESS_DENIED`.

### 2. Registry Virtualization and Redirects
* **The Mechanism:** On 64-bit systems, registry keys under `HKLM\SOFTWARE` are split into 64-bit and 32-bit views (`WOW6432Node`).
* **The Flaw:** If the driver writes to the native 64-bit registry tree, but the PC client reads from a different context or registry redirection takes place (e.g. UAC registry virtualization for non-admin processes), they will access different token buffers.

### 3. Registry Permissions / UAC
* **The Mechanism:** Writing to `HKLM\SOFTWARE` requires administrator privileges.
* **The Flaw:** If the system driver runs under a restricted context or fails to create the subkey during installation, the registry state becomes corrupt.

---

## Action Plan & Fixes

### Fix A: Modify the Driver to Create the Key and Assert Success (Recommended)
We must ensure the driver creates the registry key if it does not exist, and fails driver initialization if the token cannot be safely written:

1. **Use `ZwCreateKey`** in the driver to explicitly create the `\Registry\Machine\Software\Lampyris` key before calling `RtlWriteRegistryValue`.
2. **Handle errors** in `lampyris_core.cpp`:
   ```cpp
   NTSTATUS Status = WriteTokenToRegistry();
   if (!NT_SUCCESS(Status)) {
       // Log error or fail driver entry/device creation
   }
   ```

### Fix B: Create Registry Key in the INF Installer
Update the `ComponentizedAudioSample.inx` installer file to automatically create the registry key with appropriate ACLs (Read access for everyone, Write access for SYSTEM):
```inf
[Lampyris_Registry_Add]
HKLM,SOFTWARE\Lampyris,,0x00000010
```

### Fix C: Manual Verification & Workaround
To test immediately without recompiling the driver:
1. Open `regedit.exe` in the VM.
2. Manually create the key `HKLM\SOFTWARE\Lampyris` if it doesn't exist.
3. Restart the VM (forces the driver to reload and write the new `SessionToken`).
4. Verify the `SessionToken` binary value inside `HKLM\SOFTWARE\Lampyris` has updated timestamps or modified contents.
5. Run the PC client again.
