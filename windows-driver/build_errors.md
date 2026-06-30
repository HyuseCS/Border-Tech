# Driver Build Errors

When initially attempting to build the entire `sysvad.sln` solution using MSBuild, several subprojects related to Audio Processing Objects (APOs) and Keyword Detectors failed to compile. The core issue was missing `.h` and `.idl` header files, likely due to incomplete sample code or missing generated files from the MIDL compiler in the Sysvad sample.

Here is the summary of the errors encountered:

## 1. KWSApo (Keyword Spotting APO)
```text
D:\Border-Tech\windows-driver\lampyris-sysvad\APO\KWSApo\KWSApo.h(13,10): error C1083: Cannot open include file: 'KWSApoInterface.h': No such file or directory
D:\Border-Tech\windows-driver\lampyris-sysvad\APO\KWSApo\KWSApoDll.cpp(17,10): error C1083: Cannot open include file: 'KWSApoDll.h': No such file or directory
```

## 2. AecApo (Acoustic Echo Cancellation APO)
```text
D:\Border-Tech\windows-driver\lampyris-sysvad\APO\AecApo\AecApoDll.cpp(17,10): error C1083: Cannot open include file: 'AecApoDll.h': No such file or directory
```

## 3. KeywordDetectorContosoAdapter
```text
midl : command line error MIDL1001: cannot open input file KeywordDetectorOemAdapter.idl
```

## 4. DelayAPO
```text
D:\Border-Tech\windows-driver\lampyris-sysvad\APO\DelayAPO\DelayAPO.h(13,10): error C1083: Cannot open include file: 'DelayAPOInterface.h': No such file or directory
D:\Border-Tech\windows-driver\lampyris-sysvad\APO\DelayAPO\DelayAPODll.cpp(17,10): error C1083: Cannot open include file: 'DelayAPODll.h': No such file or directory
```

## 5. SwapAPO
```text
D:\Border-Tech\windows-driver\lampyris-sysvad\APO\SwapAPO\SwapAPO.h(14,10): error C1083: Cannot open include file: 'SwapAPOInterface.h': No such file or directory
D:\Border-Tech\windows-driver\lampyris-sysvad\APO\SwapAPO\SwapAPODll.cpp(17,10): error C1083: Cannot open include file: 'SwapAPODll.h': No such file or directory
```

## Resolution
Since these APO projects are not strictly required for the core `lampyris-mic.sys` driver, the solution was to follow the fallback instruction in `debug_capture_instructions.md`. We bypassed the top-level solution build and directly built the required components:
1. `EndpointsCommon\EndpointsCommon.vcxproj`
2. `TabletAudioSample\TabletAudioSample.vcxproj`

This cleanly produced the `EndpointsCommon.lib` and `lampyris-mic.sys` artifacts.
