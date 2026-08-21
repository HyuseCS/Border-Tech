#Requires -RunAsAdministrator
<#
.SYNOPSIS
    Build, sign, purge and install the Lampyris virtual microphone driver.

.DESCRIPTION
    One command for the full rebuild cycle. Run from an ELEVATED PowerShell
    prompt; it finds MSBuild itself, so the VS Native Tools prompt is not needed.

        powershell -ExecutionPolicy Bypass -File .\rebuild_install.ps1

    Steps:
      1. Build sysvad.sln (Release x64).
      2. Sign via sign_driver.ps1.
      3. Delete EVERY ComponentizedAudioSample package already in the driver store.
      4. Install the freshly signed package.
      5. Offer to reboot.

.PARAMETER SkipBuild
    Reuse the existing SignedPackage instead of rebuilding and re-signing.

.PARAMETER NoReboot
    Do not offer to reboot at the end.

.NOTES
    The solution ALWAYS reports "Build FAILED" with 14 errors. Five projects
    (DelayAPO, SwapAPO, KWSApo, AecApo, KeywordDetectorContosoAdapter) reference
    headers that were never vendored into this repo, have never built, and ship
    nothing. This script therefore judges the build by whether lampyris-mic.sys
    was actually produced, not by MSBuild's exit code.
#>
param(
    [switch]$SkipBuild,
    [switch]$NoReboot
)

$ErrorActionPreference = 'Stop'

$Root        = $PSScriptRoot
$Solution    = Join-Path $Root 'lampyris-sysvad\sysvad.sln'
$SysPath     = Join-Path $Root 'lampyris-sysvad\TabletAudioSample\x64\Release\lampyris-mic.sys'
$StageDir    = Join-Path $Root 'lampyris-sysvad\TabletAudioSample\x64\SignedPackage'
$StagedInf   = Join-Path $StageDir 'ComponentizedAudioSample.inf'
$SignScript  = Join-Path $Root 'sign_driver.ps1'

function Step($n, $text) { Write-Host "`n[$n] $text" -ForegroundColor Cyan }
function Ok($text)       { Write-Host "  -> $text" -ForegroundColor Green }
function Note($text)     { Write-Host "  $text" -ForegroundColor DarkGray }

# ---------------------------------------------------------------------------
# 1. Build
# ---------------------------------------------------------------------------
if (-not $SkipBuild) {
    Step 1 'Building sysvad.sln (Release x64)...'

    $msbuild = Get-Command msbuild.exe -ErrorAction SilentlyContinue
    if ($msbuild) {
        $msbuildPath = $msbuild.Source
    } else {
        $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
        if (-not (Test-Path $vswhere)) {
            throw "MSBuild not on PATH and vswhere not found at $vswhere. Run this from an x64 Native Tools Command Prompt instead."
        }
        $msbuildPath = & $vswhere -latest -products * -requires Microsoft.Component.MSBuild `
                                  -find 'MSBuild\**\Bin\MSBuild.exe' | Select-Object -First 1
        if (-not $msbuildPath) { throw 'vswhere could not locate MSBuild.exe.' }
    }
    Note "msbuild: $msbuildPath"

    $before = if (Test-Path $SysPath) { (Get-Item $SysPath).LastWriteTimeUtc } else { [datetime]::MinValue }

    $log = Join-Path $env:TEMP 'lampyris-build.log'
    & $msbuildPath $Solution /p:Configuration=Release /p:Platform=x64 /v:n `
        | Tee-Object -FilePath $log | Out-Null

    # Judge by artifact, not exit code - see .NOTES above.
    if (-not (Test-Path $SysPath)) {
        Write-Host "  lampyris-mic.sys was not produced. Real errors:" -ForegroundColor Red
        Select-String -Path $log -Pattern 'error' |
            Where-Object { $_.Line -match 'TabletAudioSample|EndpointsCommon|minipairs|micinwavtable|micarray|lampyris' } |
            ForEach-Object { Write-Host "    $($_.Line.Trim())" -ForegroundColor Red }
        throw "Driver build failed. Full log: $log"
    }
    $after = (Get-Item $SysPath).LastWriteTimeUtc
    if ($after -le $before) {
        Note 'lampyris-mic.sys unchanged - nothing to rebuild (this is fine if you changed nothing).'
    }
    Ok "lampyris-mic.sys built $((Get-Item $SysPath).LastWriteTime)"
    Note 'The 14 APO/KeywordDetector errors are expected and were ignored.'

    # -----------------------------------------------------------------------
    # 2. Sign
    # -----------------------------------------------------------------------
    Step 2 'Signing...'
    & powershell.exe -ExecutionPolicy Bypass -File $SignScript
    if ($LASTEXITCODE -ne 0) { throw "sign_driver.ps1 failed with exit code $LASTEXITCODE." }
    Ok 'Package signed.'
}

if (-not (Test-Path $StagedInf)) { throw "Signed package not found at $StagedInf. Run without -SkipBuild." }
$driverVer = (Select-String -Path $StagedInf -Pattern '^DriverVer\s*=' | Select-Object -First 1).Line.Trim()
Note "Package to install: $driverVer"

# ---------------------------------------------------------------------------
# 3. Purge every copy already in the driver store
# ---------------------------------------------------------------------------
Step 3 'Purging existing ComponentizedAudioSample packages...'
$enum = pnputil /enum-drivers | Out-String

# Blocks are separated by blank lines; keep the ones naming our INF.
$stale = $enum -split "`r?`n`r?`n" |
    Where-Object { $_ -match 'componentizedaudiosample\.inf' } |
    ForEach-Object { if ($_ -match 'Published Name:\s*(oem\d+\.inf)') { $Matches[1] } }

if (-not $stale) {
    Note 'None present.'
} else {
    foreach ($inf in $stale) {
        Note "deleting $inf"
        pnputil /delete-driver $inf /uninstall /force | Out-Null
    }
    Ok "Removed $($stale.Count) package(s)."
}

# ---------------------------------------------------------------------------
# 4. Install
# ---------------------------------------------------------------------------
Step 4 'Installing the signed package...'
pnputil /add-driver $StagedInf /install
Ok 'Installed.'

# ---------------------------------------------------------------------------
# 5. Reboot
# ---------------------------------------------------------------------------
Step 5 'Reboot required.'
Write-Host '  The driver must start at boot for the endpoint to appear correctly.' -ForegroundColor Yellow
if (-not $NoReboot) {
    $answer = Read-Host '  Reboot now? [y/N]'
    if ($answer -match '^(y|yes)$') { Restart-Computer -Force }
}
Write-Host ''
Write-Host 'After the reboot, verify in this order:' -ForegroundColor Cyan
Write-Host '  1. mmsys.cpl -> Recording: one Lampyris device'
Write-Host '  2. windows-driver\tools\wasapi_probe.exe: GetMixFormat S_OK, 1 ch, 48000 Hz'
Write-Host '  3. Phone streaming + lampyris.exe + Voice Recorder: audible, right pitch, no delay'
Write-Host '  4. Discord or Teams mic test: exercises COMMUNICATIONS mode'
Write-Host ''
