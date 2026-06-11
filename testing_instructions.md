# Next Steps: Finalizing and Testing the Fix

Because you are transferring to a different PC that has Rust installed, here is the exact sequence of steps to deploy and test the final pipeline.

## 1. Commit and Push from the Host
First, commit the changes we made to the codebase on this PC so you can pull them to your other PC. You can use the `commit_message.md` file we just created:
```bash
git commit -F commit_message.md
```

## 2. Pull and Build on the Target PC
Once you are on the target PC (which has Rust installed), pull the repository.

**Rebuild the PC Client:**
Navigate to the `pc-client` directory and compile the optimized release build:
```bash
cd pc-client
cargo build --release
```

## 3. Deploy the Driver Update to the VM
Copy the newly built driver release folder from your target PC into the VM.
- **Source:** `windows-driver\lampyris-sysvad\TabletAudioSample\x64\Release\`
- **Destination:** Anywhere inside your VM.

## 4. Install the Updated Driver
Inside the VM, force Windows to load the new kernel code:
1. Open **Device Manager**.
2. Find the existing **Lampyris Virtual Microphone**.
3. Right-click -> **Update driver**.
4. Select **Browse my computer for drivers**.
5. Select **Let me pick from a list of available drivers on my computer**.
6. Click **Have Disk...** and navigate to the `ComponentizedAudioSample.inf` you just copied over.
7. Click **Install this driver software anyway** when prompted.

## 5. Reboot and Test
1. **Reboot the VM.** (This is the critical test to ensure the driver boot initialization successfully writes the new token).
2. Start the Android `Sonus` app on your phone.
3. Bring the newly compiled `lampyris.exe` (from step 2) into the VM.
4. Run the PC client in the VM:
   ```cmd
   set SLINT_BACKEND=winit-software && lampyris.exe --debug
   ```
5. Speak into your phone. 
6. Check the **Recording** tab in the Windows Sound Settings inside the VM. The green volume meter next to the Lampyris Microphone should finally light up and bounce to your voice!
