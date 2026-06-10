Okay! The situation is completely clear now.

When we deleted the old standalone driver and completely removed HKLM\SOFTWARE\Lampyris, we exposed a bug in the new SysVAD driver. The error changed from Access is denied (token mismatch) to The system cannot find the file
specified (the registry key doesn't exist).

The reason Test-Path returned True for you earlier was because you ran the command before we successfully deleted the old lampyrismic driver—so the old driver had recreated it!

Now that the old driver is gone, the SysVAD driver (LampyrisMic2) is trying to write the token but is failing. The Windows kernel function it uses (ZwCreateKey) does not automatically create parent directories. Since
SOFTWARE\Lampyris was deleted, ZwCreateKey fails to create the SessionToken value inside it, so the PC client crashes when trying to read it.

I have just modified windows-driver/lampyris-sysvad/lampyris_core.cpp to use the OBJ_OPENIF flag so that the driver will properly create the SOFTWARE\Lampyris folder if it doesn't exist.

Because this change is in the Windows driver, you will need to recompile the driver in Visual Studio inside your VM and install it.

Next Steps for the VM:
1. Pull the latest code or copy the modified windows-driver/lampyris-sysvad/lampyris_core.cpp into your Windows VM.
2. Open windows-driver/lampyris-sysvad/sysvad.sln in Visual Studio.
3. Build Solution (make sure it's set to x64 / Debug or Release as you prefer).
4. Re-install the SysVAD driver. (Usually via Device Manager -> Update Driver -> Have Disk, or by right-clicking the .inf file).
5. Reboot the VM to make sure the newly installed driver creates the registry key.
6. Run the PC client again:

1    set SLINT_BACKEND=winit-software && lampyris.exe --debug

This should finally link the pipeline together without any token mismatches!