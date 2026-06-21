Fix: Align Capture Pin Topology and Wave Formats to 48kHz Stereo

**Root Cause:**
The Windows Virtual Audio Driver was suffering from an initialization failure (`NotReadableError` in browsers, no audio in "Listen to this device") caused by an internal format mismatch. The driver was advertising support for 48kHz Stereo in its Wave Stream formats but had its Topology Jack Descriptor (`MicInJackDesc`) still configured as `KSAUDIO_SPEAKER_MONO`. Furthermore, it advertised several legacy Mono data ranges alongside the Stereo format. When modern applications or the Windows Audio Engine attempted to initialize the audio graph, this inconsistency caused `IAudioClient::Initialize` to fail, preventing the stream from ever reaching `KSSTATE_RUN`.

**Changes Made:**
1. **Topology Alignment (`micintoptable.h`)**:
   - Updated `MicInJackDesc` to explicitly use `KSAUDIO_SPEAKER_STEREO` to match the physical output of our ring buffer.
2. **Format Enforcement (`micinwavtable.h`)**:
   - Removed all legacy and unsupported Mono format data ranges from `MicInPinSupportedDeviceFormats`.
   - The driver now exclusively advertises 48kHz Stereo format. This forces the Windows Audio Engine to handle any necessary resampling to lower sample rates or Mono channels requested by applications (like WebRTC), guaranteeing the driver always provides uncorrupted native stream data.

**Testing:**
- Driver compiles successfully via MSBuild.
- Driver loads and signs successfully.
- (Requires testing) User must verify via Sound Settings -> Advanced that the Default Format is set to 48kHz Stereo, clearing any old cached Mono configurations from the registry to allow the stream to initialize.
