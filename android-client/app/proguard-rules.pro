# R8 / ProGuard rules for the Sonus release build.
# This file exists because app/build.gradle.kts references it in proguardFiles.
# Add keep rules here if minification ever strips something the app needs at runtime.

# BouncyCastle's JCE provider registers algorithm implementations reflectively by
# class-name string (see Security.addProvider(BouncyCastleProvider()) in
# AudioCaptureService.kt). R8 cannot see those references and strips the classes,
# so the release build fails at runtime with "PKCS12 not found".
# A broad keep is required here: the reflective lookups are name-derived, so there
# is no narrower rule R8 can verify.
-keep class org.bouncycastle.** { *; }
-dontwarn org.bouncycastle.**
