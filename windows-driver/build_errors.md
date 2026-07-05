D:\Border-Tech\windows-driver>cd D:\Border-Tech\windows-driver\lampyris-sysvad

D:\Border-Tech\windows-driver\lampyris-sysvad>msbuild sysvad.sln /p:Configuration=Release /p:Platform=x64
MSBuild version 17.14.40+3e7442088 for .NET Framework
Build started 05/07/2026 11:38:11 am.

Project "D:\Border-Tech\windows-driver\lampyris-sysvad\sysvad.sln" on node 1 (default targets).
ValidateSolutionConfiguration:
  Building solution configuration "Release|x64".
Project "D:\Border-Tech\windows-driver\lampyris-sysvad\sysvad.sln" (1) is building "D:\Border-Tech\windows-driver\lampy
ris-sysvad\Package\package.VcxProj" (2) on node 1 (default targets).
Project "D:\Border-Tech\windows-driver\lampyris-sysvad\Package\package.VcxProj" (2) is building "D:\Border-Tech\windows
-driver\lampyris-sysvad\TabletAudioSample\TabletAudioSample.vcxproj" (3) on node 1 (default targets).
Project "D:\Border-Tech\windows-driver\lampyris-sysvad\TabletAudioSample\TabletAudioSample.vcxproj" (3) is building "D:
\Border-Tech\windows-driver\lampyris-sysvad\EndpointsCommon\EndpointsCommon.vcxproj" (4) on node 1 (default targets).
DriverBuildNotifications:
  Building 'EndpointsCommon' with toolset 'WindowsKernelModeDriver10.0' and the 'Windows Driver' target platform.
  Using KMDF 1.15.
PrepareForBuild:
  Structured output is enabled. The formatting of compiler diagnostics will reflect the error hierarchy. See https://ak
  a.ms/cpp/structured-output for more details.
InitializeBuildStatus:
  Creating "x64\Release\EndpointsCommon.tlog\unsuccessfulbuild" because "AlwaysCreate" was specified.
  Touching "x64\Release\EndpointsCommon.tlog\unsuccessfulbuild".
ClCompile:
  All outputs are up-to-date.
Lib:
  All outputs are up-to-date.
  EndpointsCommon.vcxproj -> D:\Border-Tech\windows-driver\lampyris-sysvad\EndpointsCommon\x64\Release\EndpointsCommon.
  lib
FinalizeBuildStatus:
  Deleting file "x64\Release\EndpointsCommon.tlog\unsuccessfulbuild".
  Touching "x64\Release\EndpointsCommon.tlog\EndpointsCommon.lastbuildstate".
Done Building Project "D:\Border-Tech\windows-driver\lampyris-sysvad\EndpointsCommon\EndpointsCommon.vcxproj" (default
targets).

DriverBuildNotifications:
  Building 'TabletAudioSample' with toolset 'WindowsKernelModeDriver10.0' and the 'Universal' target platform.
  Using KMDF 1.15.
PrepareForBuild:
  Structured output is enabled. The formatting of compiler diagnostics will reflect the error hierarchy. See https://ak
  a.ms/cpp/structured-output for more details.
InitializeBuildStatus:
  Creating "x64\Release\TabletAu.E4DF0EEE.tlog\unsuccessfulbuild" because "AlwaysCreate" was specified.
  Touching "x64\Release\TabletAu.E4DF0EEE.tlog\unsuccessfulbuild".
StampInf:
  C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x86\stampinf.exe -d "*" -a "amd64" -v "*" -k "1.15"  -x -f x6
  4\Release\ComponentizedApoSample.inf
  Copying "D:\Border-Tech\windows-driver\lampyris-sysvad\TabletAudioSample\ComponentizedApoSample.inx" to "x64\Release\
  ComponentizedApoSample.inf" for stamping
  Stamping x64\Release\ComponentizedApoSample.inf
  Stamping [Version] section with DriverVer=07/05/2026,11.38.12.475
  C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x86\stampinf.exe -d "*" -a "amd64" -v "*" -k "1.15"  -x -f x6
  4\Release\ComponentizedAudioSample.inf
  Copying "D:\Border-Tech\windows-driver\lampyris-sysvad\TabletAudioSample\ComponentizedAudioSample.inx" to "x64\Releas
  e\ComponentizedAudioSample.inf" for stamping
  Stamping x64\Release\ComponentizedAudioSample.inf
  Stamping [Version] section with DriverVer=07/05/2026,11.38.12.560
  C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x86\stampinf.exe -d "*" -a "amd64" -v "*" -k "1.15"  -x -f x6
  4\Release\ComponentizedAudioSampleExtension.inf
  Copying "D:\Border-Tech\windows-driver\lampyris-sysvad\TabletAudioSample\ComponentizedAudioSampleExtension.inx" to "x
  64\Release\ComponentizedAudioSampleExtension.inf" for stamping
  Stamping x64\Release\ComponentizedAudioSampleExtension.inf
  Stamping [Version] section with DriverVer=07/05/2026,11.38.12.632
D:\Border-Tech\windows-driver\lampyris-sysvad\TabletAudioSample\ComponentizedAudioSample.inx(43-43): warning 2083: Sect
ion [keyworddetectorcontosoadapter.copylist] not referenced or used. [D:\Border-Tech\windows-driver\lampyris-sysvad\Tab
letAudioSample\TabletAudioSample.vcxproj]
ClCompile:
  All outputs are up-to-date.
ResourceCompile:
  All outputs are up-to-date.
Link:
  All outputs are up-to-date.
  TabletAudioSample.vcxproj -> D:\Border-Tech\windows-driver\lampyris-sysvad\TabletAudioSample\x64\Release\lampyris-mic
  .sys
Project "D:\Border-Tech\windows-driver\lampyris-sysvad\TabletAudioSample\TabletAudioSample.vcxproj" (3) is building "D:
\Border-Tech\windows-driver\lampyris-sysvad\TabletAudioSample\TabletAudioSample.vcxproj" (3:2) on node 1 (TestSign targ
et(s)).
TestSign:
  The driver will be test-signed. Driver signing options can be changed from the project properties.
  Sign Inputs: D:\Border-Tech\windows-driver\lampyris-sysvad\TabletAudioSample\x64\Release\lampyris-mic.sys
  All outputs are up-to-date.
Done Building Project "D:\Border-Tech\windows-driver\lampyris-sysvad\TabletAudioSample\TabletAudioSample.vcxproj" (Test
Sign target(s)).

ApiValidator:
  Validating 'Universal' driver using ApiValidator.exe
  cmd.exe /D /C "C:\Users\Hyuse\AppData\Local\Temp\MSBuildTemp\tmpd6becda6d12648ba8122c0ddbbfbd46f.cmd"
  "C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\ApiValidator.exe" -DriverPackagePath:x64\Release\lampyri
  s-mic.sys -SupportedApiXmlFiles:"C:\Program Files (x86)\Windows Kits\10\build\10.0.22621.0\universalDDIs\x64\Universa
  lDDIs.xml" -ModuleWhiteListXmlFiles:"C:\Program Files (x86)\Windows Kits\10\build\10.0.22621.0\universalDDIs\x64\Modu
  leWhiteList.xml" -ApiExtractorExePath:"C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64"
  Driver is 'Universal'.
FinalizeBuildStatus:
  Deleting file "x64\Release\TabletAu.E4DF0EEE.tlog\unsuccessfulbuild".
  Touching "x64\Release\TabletAu.E4DF0EEE.tlog\TabletAudioSample.lastbuildstate".
Done Building Project "D:\Border-Tech\windows-driver\lampyris-sysvad\TabletAudioSample\TabletAudioSample.vcxproj" (defa
ult targets).

PrepareForBuild:
  Structured output is enabled. The formatting of compiler diagnostics will reflect the error hierarchy. See https://ak
  a.ms/cpp/structured-output for more details.
InitializeBuildStatus:
  Creating "x64\Release\package.tlog\unsuccessfulbuild" because "AlwaysCreate" was specified.
  Touching "x64\Release\package.tlog\unsuccessfulbuild".
FinalizeBuildStatus:
  Deleting file "x64\Release\package.tlog\unsuccessfulbuild".
  Touching "x64\Release\package.tlog\package.lastbuildstate".
Done Building Project "D:\Border-Tech\windows-driver\lampyris-sysvad\Package\package.VcxProj" (default targets).

Project "D:\Border-Tech\windows-driver\lampyris-sysvad\sysvad.sln" (1) is building "D:\Border-Tech\windows-driver\lampy
ris-sysvad\KeywordDetectorAdapter\KeywordDetectorContosoAdapter.vcxproj" (5) on node 1 (default targets).
PrepareForBuild:
  Structured output is enabled. The formatting of compiler diagnostics will reflect the error hierarchy. See https://ak
  a.ms/cpp/structured-output for more details.
InitializeBuildStatus:
  Touching "x64\Release\KeywordD.E0F02048.tlog\unsuccessfulbuild".
Midl:
  C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\midl.exe /D _WINDLL /D _USRDLL /D UNICODE /D _UNICODE /I.
  .\inc /I..\ /W1 /nologo /char signed /env x64 /h "KeywordDetectorContosoAdapter_h.h" /tlb "x64\Release\KeywordDetecto
  rContosoAdapter.tlb" /target "NT60" KeywordDetectorContosoAdapter.idl
  64 bit Processing .\KeywordDetectorContosoAdapter.idl
  KeywordDetectorContosoAdapter.idl
  64 bit Processing C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\um\oaidl.idl
  oaidl.idl
  64 bit Processing C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\um\objidl.idl
  objidl.idl
  64 bit Processing C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\um\unknwn.idl
  unknwn.idl
  64 bit Processing C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\shared\wtypes.idl
  wtypes.idl
  64 bit Processing C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\shared\wtypesbase.idl
  wtypesbase.idl
  64 bit Processing C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\shared\basetsd.h
  basetsd.h
  64 bit Processing C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\shared\guiddef.h
  guiddef.h
  64 bit Processing C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\um\ocidl.idl
  ocidl.idl
  64 bit Processing C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\um\oleidl.idl
  oleidl.idl
  64 bit Processing C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\um\servprov.idl
  servprov.idl
  64 bit Processing C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\um\urlmon.idl
  urlmon.idl
  64 bit Processing C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\um\msxml.idl
  msxml.idl
midl : command line error MIDL1001: cannot open input file KeywordDetectorOemAdapter.idl [D:\Border-Tech\windows-driver
\lampyris-sysvad\KeywordDetectorAdapter\KeywordDetectorContosoAdapter.vcxproj]
Done Building Project "D:\Border-Tech\windows-driver\lampyris-sysvad\KeywordDetectorAdapter\KeywordDetectorContosoAdapt
er.vcxproj" (default targets) -- FAILED.

Project "D:\Border-Tech\windows-driver\lampyris-sysvad\sysvad.sln" (1) is building "D:\Border-Tech\windows-driver\lampy
ris-sysvad\APO\DelayAPO\DelayAPO.vcxproj" (7) on node 1 (default targets).
PrepareForBuild:
  Structured output is enabled. The formatting of compiler diagnostics will reflect the error hierarchy. See https://ak
  a.ms/cpp/structured-output for more details.
InitializeBuildStatus:
  Touching "x64\Release\DelayAPO.tlog\unsuccessfulbuild".
Midl:
  All outputs are up-to-date.
  All outputs are up-to-date.
ClCompile:
  C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\HostX64\x64\CL.exe /c /I..\inc
  /I..\..\ /I. /Zi /nologo /W4 /WX /diagnostics:column /O2 /D _WINDLL /D _WINDLL /D _USRDLL /D UNICODE /D _UNICODE /Gm-
   /GS /fp:precise /Zc:wchar_t /Zc:forScope /Zc:inline /std:c++17 /Fo"x64\Release\\" /Fd"x64\Release\vc143.pdb" /extern
  al:W4 /Gd /TP /FC /errorReport:queue Delay.cpp DelayAPODll.cpp DelayAPOMFX.cpp DelayAPOSFX.cpp
  Delay.cpp
  DelayAPODll.cpp
D:\Border-Tech\windows-driver\lampyris-sysvad\APO\DelayAPO\DelayAPO.h(13,10): error C1083: Cannot open include file: 'D
elayAPOInterface.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\DelayAPO\DelayAPO.vcx
proj]
  (compiling source file 'Delay.cpp')

D:\Border-Tech\windows-driver\lampyris-sysvad\APO\DelayAPO\DelayAPODll.cpp(17,10): error C1083: Cannot open include fil
e: 'DelayAPODll.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\DelayAPO\DelayAPO.vcxp
roj]
  DelayAPOMFX.cpp
D:\Border-Tech\windows-driver\lampyris-sysvad\APO\DelayAPO\DelayAPO.h(13,10): error C1083: Cannot open include file: 'D
elayAPOInterface.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\DelayAPO\DelayAPO.vcx
proj]
  (compiling source file 'DelayAPOMFX.cpp')

  DelayAPOSFX.cpp
D:\Border-Tech\windows-driver\lampyris-sysvad\APO\DelayAPO\DelayAPO.h(13,10): error C1083: Cannot open include file: 'D
elayAPOInterface.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\DelayAPO\DelayAPO.vcx
proj]
  (compiling source file 'DelayAPOSFX.cpp')

  Generating Code...
Done Building Project "D:\Border-Tech\windows-driver\lampyris-sysvad\APO\DelayAPO\DelayAPO.vcxproj" (default targets) -
- FAILED.

Project "D:\Border-Tech\windows-driver\lampyris-sysvad\sysvad.sln" (1) is building "D:\Border-Tech\windows-driver\lampy
ris-sysvad\APO\SwapAPO\SwapAPO.vcxproj" (8) on node 1 (default targets).
PrepareForBuild:
  Structured output is enabled. The formatting of compiler diagnostics will reflect the error hierarchy. See https://ak
  a.ms/cpp/structured-output for more details.
InitializeBuildStatus:
  Touching "x64\Release\SwapAPO.tlog\unsuccessfulbuild".
Midl:
  All outputs are up-to-date.
  All outputs are up-to-date.
ClCompile:
  C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\HostX64\x64\CL.exe /c /I..\inc
  /I..\..\ /I. /I..\..\..\..\wil\include /Zi /nologo /W4 /WX /diagnostics:column /O2 /D _WINDLL /D _WINDLL /D _USRDLL /
  D UNICODE /D _UNICODE /Gm- /GS /fp:precise /Zc:wchar_t /Zc:forScope /Zc:inline /std:c++17 /Fo"x64\Release\\" /Fd"x64\
  Release\vc143.pdb" /external:W4 /Gd /TP /FC /errorReport:queue Swap.cpp SwapAPODll.cpp SwapAPOMFX.cpp SwapAPOSFX.cpp
  Swap.cpp
D:\Border-Tech\windows-driver\lampyris-sysvad\APO\SwapAPO\SwapAPO.h(14,10): error C1083: Cannot open include file: 'Swa
pAPOInterface.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\SwapAPO\SwapAPO.vcxproj]
  (compiling source file 'Swap.cpp')

  SwapAPODll.cpp
D:\Border-Tech\windows-driver\lampyris-sysvad\APO\SwapAPO\SwapAPODll.cpp(17,10): error C1083: Cannot open include file:
 'SwapAPODll.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\SwapAPO\SwapAPO.vcxproj]
  SwapAPOMFX.cpp
D:\Border-Tech\windows-driver\lampyris-sysvad\APO\SwapAPO\SwapAPO.h(14,10): error C1083: Cannot open include file: 'Swa
pAPOInterface.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\SwapAPO\SwapAPO.vcxproj]
  (compiling source file 'SwapAPOMFX.cpp')

  SwapAPOSFX.cpp
D:\Border-Tech\windows-driver\lampyris-sysvad\APO\SwapAPO\SwapAPO.h(14,10): error C1083: Cannot open include file: 'Swa
pAPOInterface.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\SwapAPO\SwapAPO.vcxproj]
  (compiling source file 'SwapAPOSFX.cpp')

  Generating Code...
Done Building Project "D:\Border-Tech\windows-driver\lampyris-sysvad\APO\SwapAPO\SwapAPO.vcxproj" (default targets) --
FAILED.

Project "D:\Border-Tech\windows-driver\lampyris-sysvad\sysvad.sln" (1) is building "D:\Border-Tech\windows-driver\lampy
ris-sysvad\APO\KWSApo\KwsAPO.vcxproj" (9) on node 1 (default targets).
PrepareForBuild:
  Structured output is enabled. The formatting of compiler diagnostics will reflect the error hierarchy. See https://ak
  a.ms/cpp/structured-output for more details.
InitializeBuildStatus:
  Touching "x64\Release\KwsAPO.tlog\unsuccessfulbuild".
Midl:
  All outputs are up-to-date.
  All outputs are up-to-date.
ClCompile:
  C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\HostX64\x64\CL.exe /c /I..\inc
  /I..\..\ /I. /I..\..\..\..\wil\include /Zi /nologo /W4 /WX /diagnostics:column /O2 /D _WINDLL /D _WINDLL /D _USRDLL /
  D UNICODE /D _UNICODE /Gm- /GS /fp:precise /Zc:wchar_t /Zc:forScope /Zc:inline /std:c++17 /Fo"x64\Release\\" /Fd"x64\
  Release\vc143.pdb" /external:W4 /Gd /TP /FC /errorReport:queue KWSApo.cpp KWSApoDll.cpp KWSApoEFX.cpp
  KWSApo.cpp
D:\Border-Tech\windows-driver\lampyris-sysvad\APO\KWSApo\KWSApo.h(13,10): error C1083: Cannot open include file: 'KWSAp
oInterface.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\KWSApo\KwsAPO.vcxproj]
  (compiling source file 'KWSApo.cpp')

  KWSApoDll.cpp
D:\Border-Tech\windows-driver\lampyris-sysvad\APO\KWSApo\KWSApoDll.cpp(17,10): error C1083: Cannot open include file: '
KWSApoDll.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\KWSApo\KwsAPO.vcxproj]
  KWSApoEFX.cpp
D:\Border-Tech\windows-driver\lampyris-sysvad\APO\KWSApo\KWSApo.h(13,10): error C1083: Cannot open include file: 'KWSAp
oInterface.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\KWSApo\KwsAPO.vcxproj]
  (compiling source file 'KWSApoEFX.cpp')

  Generating Code...
Done Building Project "D:\Border-Tech\windows-driver\lampyris-sysvad\APO\KWSApo\KwsAPO.vcxproj" (default targets) -- FA
ILED.

Project "D:\Border-Tech\windows-driver\lampyris-sysvad\sysvad.sln" (1) is building "D:\Border-Tech\windows-driver\lampy
ris-sysvad\APO\AecApo\AecAPO.vcxproj" (10) on node 1 (default targets).
PrepareForBuild:
  Structured output is enabled. The formatting of compiler diagnostics will reflect the error hierarchy. See https://ak
  a.ms/cpp/structured-output for more details.
InitializeBuildStatus:
  Touching "x64\Release\AecAPO.tlog\unsuccessfulbuild".
Midl:
  All outputs are up-to-date.
ClCompile:
  C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\HostX64\x64\CL.exe /c /I..\inc
  /I..\..\ /I. /I..\..\..\..\wil\include /Zi /nologo /W4 /WX /diagnostics:column /O2 /D _WINDLL /D _WINDLL /D _USRDLL /
  D UNICODE /D _UNICODE /Gm- /GS /fp:precise /Zc:wchar_t /Zc:forScope /Zc:inline /std:c++17 /Fo"x64\Release\\" /Fd"x64\
  Release\vc143.pdb" /external:W4 /Gd /TP /FC /errorReport:queue AecApoDll.cpp AecApoMFX.cpp
  AecApoDll.cpp
D:\Border-Tech\windows-driver\lampyris-sysvad\APO\AecApo\AecApoDll.cpp(17,10): error C1083: Cannot open include file: '
AecApoDll.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\AecApo\AecAPO.vcxproj]
  AecApoMFX.cpp
D:\Border-Tech\windows-driver\lampyris-sysvad\APO\AecApo\AecApo.h(13,10): error C1083: Cannot open include file: 'AecAp
oDll.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\AecApo\AecAPO.vcxproj]
  (compiling source file 'AecApoMFX.cpp')

  Generating Code...
Done Building Project "D:\Border-Tech\windows-driver\lampyris-sysvad\APO\AecApo\AecAPO.vcxproj" (default targets) -- FA
ILED.

Done Building Project "D:\Border-Tech\windows-driver\lampyris-sysvad\sysvad.sln" (default targets) -- FAILED.


Build FAILED.

"D:\Border-Tech\windows-driver\lampyris-sysvad\sysvad.sln" (default target) (1) ->
"D:\Border-Tech\windows-driver\lampyris-sysvad\Package\package.VcxProj" (default target) (2) ->
"D:\Border-Tech\windows-driver\lampyris-sysvad\TabletAudioSample\TabletAudioSample.vcxproj" (default target) (3) ->
(InfVerif target) ->
  D:\Border-Tech\windows-driver\lampyris-sysvad\TabletAudioSample\ComponentizedAudioSample.inx(43-43): warning 2083: Se
ction [keyworddetectorcontosoadapter.copylist] not referenced or used. [D:\Border-Tech\windows-driver\lampyris-sysvad\T
abletAudioSample\TabletAudioSample.vcxproj]


"D:\Border-Tech\windows-driver\lampyris-sysvad\sysvad.sln" (default target) (1) ->
"D:\Border-Tech\windows-driver\lampyris-sysvad\KeywordDetectorAdapter\KeywordDetectorContosoAdapter.vcxproj" (default t
arget) (5) ->
(Midl target) ->
  midl : command line error MIDL1001: cannot open input file KeywordDetectorOemAdapter.idl [D:\Border-Tech\windows-driv
er\lampyris-sysvad\KeywordDetectorAdapter\KeywordDetectorContosoAdapter.vcxproj]


"D:\Border-Tech\windows-driver\lampyris-sysvad\sysvad.sln" (default target) (1) ->
"D:\Border-Tech\windows-driver\lampyris-sysvad\APO\DelayAPO\DelayAPO.vcxproj" (default target) (7) ->
(ClCompile target) ->
  D:\Border-Tech\windows-driver\lampyris-sysvad\APO\DelayAPO\DelayAPO.h(13,10): error C1083: Cannot open include file:
'DelayAPOInterface.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\DelayAPO\DelayAPO.v
cxproj]
  D:\Border-Tech\windows-driver\lampyris-sysvad\APO\DelayAPO\DelayAPODll.cpp(17,10): error C1083: Cannot open include f
ile: 'DelayAPODll.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\DelayAPO\DelayAPO.vc
xproj]
  D:\Border-Tech\windows-driver\lampyris-sysvad\APO\DelayAPO\DelayAPO.h(13,10): error C1083: Cannot open include file:
'DelayAPOInterface.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\DelayAPO\DelayAPO.v
cxproj]
  D:\Border-Tech\windows-driver\lampyris-sysvad\APO\DelayAPO\DelayAPO.h(13,10): error C1083: Cannot open include file:
'DelayAPOInterface.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\DelayAPO\DelayAPO.v
cxproj]


"D:\Border-Tech\windows-driver\lampyris-sysvad\sysvad.sln" (default target) (1) ->
"D:\Border-Tech\windows-driver\lampyris-sysvad\APO\SwapAPO\SwapAPO.vcxproj" (default target) (8) ->
  D:\Border-Tech\windows-driver\lampyris-sysvad\APO\SwapAPO\SwapAPO.h(14,10): error C1083: Cannot open include file: 'S
wapAPOInterface.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\SwapAPO\SwapAPO.vcxpro
j]
  D:\Border-Tech\windows-driver\lampyris-sysvad\APO\SwapAPO\SwapAPODll.cpp(17,10): error C1083: Cannot open include fil
e: 'SwapAPODll.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\SwapAPO\SwapAPO.vcxproj
]
  D:\Border-Tech\windows-driver\lampyris-sysvad\APO\SwapAPO\SwapAPO.h(14,10): error C1083: Cannot open include file: 'S
wapAPOInterface.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\SwapAPO\SwapAPO.vcxpro
j]
  D:\Border-Tech\windows-driver\lampyris-sysvad\APO\SwapAPO\SwapAPO.h(14,10): error C1083: Cannot open include file: 'S
wapAPOInterface.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\SwapAPO\SwapAPO.vcxpro
j]


"D:\Border-Tech\windows-driver\lampyris-sysvad\sysvad.sln" (default target) (1) ->
"D:\Border-Tech\windows-driver\lampyris-sysvad\APO\KWSApo\KwsAPO.vcxproj" (default target) (9) ->
  D:\Border-Tech\windows-driver\lampyris-sysvad\APO\KWSApo\KWSApo.h(13,10): error C1083: Cannot open include file: 'KWS
ApoInterface.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\KWSApo\KwsAPO.vcxproj]
  D:\Border-Tech\windows-driver\lampyris-sysvad\APO\KWSApo\KWSApoDll.cpp(17,10): error C1083: Cannot open include file:
 'KWSApoDll.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\KWSApo\KwsAPO.vcxproj]
  D:\Border-Tech\windows-driver\lampyris-sysvad\APO\KWSApo\KWSApo.h(13,10): error C1083: Cannot open include file: 'KWS
ApoInterface.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\KWSApo\KwsAPO.vcxproj]


"D:\Border-Tech\windows-driver\lampyris-sysvad\sysvad.sln" (default target) (1) ->
"D:\Border-Tech\windows-driver\lampyris-sysvad\APO\AecApo\AecAPO.vcxproj" (default target) (10) ->
  D:\Border-Tech\windows-driver\lampyris-sysvad\APO\AecApo\AecApoDll.cpp(17,10): error C1083: Cannot open include file:
 'AecApoDll.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\AecApo\AecAPO.vcxproj]
  D:\Border-Tech\windows-driver\lampyris-sysvad\APO\AecApo\AecApo.h(13,10): error C1083: Cannot open include file: 'Aec
ApoDll.h': No such file or directory [D:\Border-Tech\windows-driver\lampyris-sysvad\APO\AecApo\AecAPO.vcxproj]

    1 Warning(s)
    14 Error(s)

Time Elapsed 00:00:10.11