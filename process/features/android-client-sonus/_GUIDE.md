# android-client-sonus

<!-- Part of Project-M -->

## Scope

Sonus, the Android microphone app (`android-client/`, `com.projectm.mic` v1.1.0). Covers the Jetpack Compose "Cinema Mobile" UI, the foreground capture service, `AudioRecord` PCM capture at 48kHz mono s16le, the TLS **server** (the phone listens; the PC dials in), certificate generation, and SRP pairing on the device side.

Does NOT cover anything on the desktop. Note the flipped architecture: this app is the server, not the client.

## Key Source Files

- `android-client/app/src/main/java/com/projectm/mic/AudioCaptureService.kt` (550 loc) — foreground service, TLS server socket on port 47999, `AudioRecord` capture loop, `'MC'` frame emission
- `android-client/app/src/main/java/com/projectm/mic/MainActivity.kt` (485 loc) — Compose UI, local-IP display, pairing PIN, status/meter visuals
- `android-client/app/src/main/AndroidManifest.xml` — permissions and the `microphone` foreground-service type
- `android-client/app/build.gradle.kts` — SDK levels, R8 minification, APK renamed to `Sonus-v{versionName}.apk`
- `android-client/gradle/libs.versions.toml` — version catalog (Kotlin 2.1.0, AGP 8.4.1, Compose BOM 2024.05.02)

## Related Context

- `process/context/all-context.md` — wire contract (`'MC'` framing, 48kHz s16le mono), design principles from `PRODUCT.md`
- `process/context/tests/all-tests.md` — **there are no Android tests**; `./gradlew test` is a compile check only

## Gotchas

- `android-client/app/build/` is **committed to git** and is dirty after every build. Never stage it, never diff it, never "clean it up".
- There is no `src/test/` or `src/androidTest/`. Both source files are entirely unverified by automation.
- Release builds are minified — add ProGuard keep rules in `proguard-rules.pro` when adding reflection-based code.

## Current Status

Status: stable

The capture and streaming path works and has been verified end to end against both the Linux (PipeWire) and Windows (IOCTL) backends. Listed as a high-risk area because latency and dropout regressions surface here first.

## Folder Contents

```
process/features/android-client-sonus/
  active/       -- in-progress plans for this feature (each task lives inside a {slug}_{date}/ task folder)
  completed/    -- archived completed plans
  backlog/      -- deferred/future plans
```

All artifacts (plans, specs, reports, references) colocate inside each `{slug}_{date}/` task folder. Do NOT create `reports/` or `references/` sibling dirs.
