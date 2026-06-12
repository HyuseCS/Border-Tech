Fix(Driver/PC-Client): Resolve Session Token handshake registry mismatch on reboot

- driver: Replaced fragile ZwCreateKey implementation with RtlWriteRegistryValue in `WriteTokenToRegistry` to guarantee the session token is reliably written during early boot initialization regardless of missing parent hierarchies or boot-phase ACLs.
- pc-client: Appended `KEY_WOW64_64KEY` flag to `RegOpenKeyExW` in the audio backend to explicitly bypass 32-bit WOW64 registry redirection. This ensures the client reads the native 64-bit HKLM hive where the kernel driver writes the token.
