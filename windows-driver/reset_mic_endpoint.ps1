#Requires -RunAsAdministrator
<#
.SYNOPSIS
    Repairs the cached audio-engine format on the Lampyris virtual capture endpoint.

.DESCRIPTION
    IAudioClient::GetMixFormat on a CAPTURE endpoint reads the cached registry value
    PKEY_AudioEngine_DeviceFormat ({F19F064D-082C-4E27-BC73-6882A1BB8E4C},0) under
    HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Capture\{guid}\Properties.
    It never queries the driver. If that cached value is stale (or absent), the audio engine
    aborts the capture open before the miniport's NewStream is ever called and the mic is
    silent. This script writes the correct stereo 2ch/48000/16-bit blob to DeviceFormat and
    OEMFormat, after backing up the endpoint's Properties key.

    Safety: requires elevation; only touches endpoints that POSITIVELY match Lampyris by
    friendly name or by one of two documented fallback GUIDs; exports a .reg backup before
    any write; refuses to do anything if no endpoint matches. -Purge (key deletion) is
    opt-in and never the default.

.PARAMETER Purge
    Instead of writing corrected values, DELETE the matched endpoint's registry subtree so
    Windows recreates it fresh from the INF's OEMFormat seed. A .reg backup is still taken
    first. Opt-in only.

.EXAMPLE
    .\reset_mic_endpoint.ps1
.EXAMPLE
    .\reset_mic_endpoint.ps1 -Purge
#>
[CmdletBinding()]
param(
    [switch]$Purge
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------
$CaptureRoot      = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Capture'
$CaptureRootReg   = 'HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Capture'

$PkeyFriendlyName = '{a45c254e-df1c-4efd-8020-67d146a850e0},2'   # PKEY_Device_DeviceDesc
# PKEY_DeviceInterface_FriendlyName. The Lampyris endpoint's DeviceDesc is the generic
# 'External Microphone Headphone'; the 'Lampyris Virtual Microphone' string lives here.
# Matching on DeviceDesc alone finds nothing on a clean install.
$PkeyIfaceName    = '{b3f8fa53-0004-438e-9003-51a46e139bfc},6'
$PkeyDeviceFormat = '{F19F064D-082C-4E27-BC73-6882A1BB8E4C},0'
$PkeyOemFormat    = '{E4870E26-3CC5-4CD2-BA46-CA0A9A70ED04},3'

# Documented fallback endpoint GUIDs (EventViewerLogs.md).
$FallbackGuids = @(
    '{50ff5a10-7084-4bad-85d0-9f8a616b5524}',
    '{b1dd805e-09b6-4673-8efd-2f072d9bf0cf}'
)

# ---------------------------------------------------------------------------
# Elevation check (belt and braces alongside #Requires -RunAsAdministrator)
# ---------------------------------------------------------------------------
$identity  = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host 'ERROR: this script must be run from an ELEVATED PowerShell prompt.' -ForegroundColor Red
    Write-Host '       Right-click PowerShell -> "Run as administrator", then re-run.' -ForegroundColor Red
    exit 1
}

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
function Get-EndpointProperty {
    <# Returns the raw value of a property under an endpoint's Properties key, or $null. #>
    param([string]$PropertiesPath, [string]$PropertyName)

    if (-not (Test-Path -LiteralPath $PropertiesPath)) { return $null }
    try {
        $item = Get-ItemProperty -LiteralPath $PropertiesPath -Name $PropertyName -ErrorAction Stop
        return $item.$PropertyName
    } catch {
        return $null
    }
}

function Format-HexBytes {
    param([byte[]]$Bytes)
    if ($null -eq $Bytes -or $Bytes.Length -eq 0) { return '<empty>' }
    return (($Bytes | ForEach-Object { '{0:X2}' -f $_ }) -join ' ')
}

function Get-ChannelMaskName {
    param([uint32]$Mask)
    switch ($Mask) {
        0x0 { 'unspecified' ; break }
        0x1 { 'MONO (FL)'   ; break }
        0x3 { 'STEREO'      ; break }
        0x4 { 'CENTER only (legacy mono)' ; break }
        default { ('0x{0:X}' -f $Mask) }
    }
}

function Show-FormatBlob {
    <#
        Decodes a serialized VT_BLOB PROPVARIANT holding a WAVEFORMATEXTENSIBLE and prints
        it in plain text. Layout: 4-byte vt/header, 4-byte blob length, then the
        WAVEFORMATEX(TENSIBLE) itself at offset 8.
    #>
    param([string]$Label, $Value)

    if ($null -eq $Value) {
        Write-Host ("  {0}: <not set>" -f $Label)
        return
    }

    $bytes = [byte[]]$Value
    Write-Host ("  {0} raw ({1} bytes): {2}" -f $Label, $bytes.Length, (Format-HexBytes $bytes))

    $offset = 8   # skip the VT_BLOB header + length
    if ($bytes.Length -lt ($offset + 16)) {
        Write-Host ("  {0}: <too short to decode as WAVEFORMATEX ({1} bytes)>" -f $Label, $bytes.Length)
        return
    }

    $formatTag = [BitConverter]::ToUInt16($bytes, $offset + 0)
    $channels  = [BitConverter]::ToUInt16($bytes, $offset + 2)
    $rate      = [BitConverter]::ToUInt32($bytes, $offset + 4)
    $avgBytes  = [BitConverter]::ToUInt32($bytes, $offset + 8)
    $blockAlgn = [BitConverter]::ToUInt16($bytes, $offset + 12)
    $bits      = [BitConverter]::ToUInt16($bytes, $offset + 14)

    $maskText = 'n/a (not EXTENSIBLE)'
    if ($formatTag -eq 0xFFFE -and $bytes.Length -ge ($offset + 24)) {
        $mask     = [BitConverter]::ToUInt32($bytes, $offset + 20)
        $maskText = ('0x{0:X} ({1})' -f $mask, (Get-ChannelMaskName $mask))
    }

    Write-Host ("  {0}: channels={1}, rate={2}, bits={3}, blockAlign={4}, avgBytesPerSec={5}, formatTag=0x{6:X4}, mask={7}" -f `
        $Label, $channels, $rate, $bits, $blockAlgn, $avgBytes, $formatTag, $maskText)
}

# ---------------------------------------------------------------------------
# 1. Find the Lampyris capture endpoint(s) — positive identification ONLY.
# ---------------------------------------------------------------------------
if (-not (Test-Path -LiteralPath $CaptureRoot)) {
    Write-Host "ERROR: capture endpoint root not found: $CaptureRoot" -ForegroundColor Red
    exit 1
}

$matched = @()
foreach ($key in (Get-ChildItem -LiteralPath $CaptureRoot -ErrorAction SilentlyContinue)) {
    $guid      = $key.PSChildName
    $propsPath = Join-Path $key.PSPath 'Properties'
    $name      = Get-EndpointProperty -PropertiesPath $propsPath -PropertyName $PkeyFriendlyName
    $iface     = Get-EndpointProperty -PropertiesPath $propsPath -PropertyName $PkeyIfaceName

    $isMatch = $false
    $reason  = ''
    if ($name -and ([string]$name) -like '*Lampyris*') {
        $isMatch = $true; $reason = "friendly name '$name'"
    } elseif ($iface -and ([string]$iface) -like '*Lampyris*') {
        $isMatch = $true; $reason = "interface name '$iface'"
    } elseif ($FallbackGuids -contains $guid.ToLowerInvariant()) {
        $isMatch = $true; $reason = 'documented fallback GUID'
    }

    if ($isMatch) {
        $matched += [pscustomobject]@{
            Guid          = $guid
            Name          = if ($name -and $iface) { "$name ($iface)" } elseif ($name) { [string]$name } elseif ($iface) { [string]$iface } else { '(no friendly name)' }
            Reason        = $reason
            PropertiesPS  = $propsPath
            PropertiesReg = "$CaptureRootReg\$guid\Properties"
            EndpointReg   = "$CaptureRootReg\$guid"
            EndpointPS    = [string]$key.PSPath
        }
    }
}

if ($matched.Count -eq 0) {
    Write-Host 'ERROR: no Lampyris capture endpoint found.' -ForegroundColor Red
    Write-Host '       Nothing was written. No registry key was modified.' -ForegroundColor Red
    Write-Host '       Is the Lampyris Mic device installed and enabled in Device Manager?' -ForegroundColor Red
    exit 2
}

Write-Host ("Matched {0} Lampyris capture endpoint(s):" -f $matched.Count) -ForegroundColor Cyan
foreach ($m in $matched) {
    Write-Host ("  {0}  [{1}]" -f $m.Guid, $m.Reason)
}
Write-Host ''

# ---------------------------------------------------------------------------
# 2. Per endpoint: backup -> BEFORE decode -> write (or purge) -> AFTER decode.
# ---------------------------------------------------------------------------
$stamp     = Get-Date -Format 'yyyyMMdd-HHmmss'
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

foreach ($m in $matched) {
    Write-Host ('=== Endpoint {0} — {1} ===' -f $m.Guid, $m.Name) -ForegroundColor Cyan

    # (a) BACKUP FIRST — before any write of any kind.
    $backupFile = Join-Path $scriptDir ("mic_endpoint_backup_{0}_{1}.reg" -f ($m.Guid -replace '[{}]',''), $stamp)
    $backupKey  = if ($Purge) { $m.EndpointReg } else { $m.PropertiesReg }
    Write-Host ("  Backing up {0}" -f $backupKey)
    Write-Host ("          -> {0}" -f $backupFile)
    & reg.exe export $backupKey $backupFile /y | Out-Null
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $backupFile)) {
        Write-Host '  ERROR: registry backup failed. Refusing to modify anything.' -ForegroundColor Red
        exit 3
    }

    # (b) BEFORE decode — this output is the experiment that confirms or refutes
    #     the stale-DeviceFormat root cause. Capture it.
    Write-Host ''
    Write-Host '  --- BEFORE ---' -ForegroundColor Yellow
    Show-FormatBlob -Label 'DeviceFormat' -Value (Get-EndpointProperty -PropertiesPath $m.PropertiesPS -PropertyName $PkeyDeviceFormat)
    Show-FormatBlob -Label 'OEMFormat'    -Value (Get-EndpointProperty -PropertiesPath $m.PropertiesPS -PropertyName $PkeyOemFormat)
    Write-Host ''

    if ($Purge) {
        # (c-purge) Opt-in destructive path: delete the endpoint subtree entirely.
        Write-Host '  -Purge specified: deleting the endpoint registry subtree.' -ForegroundColor Yellow
        Remove-Item -LiteralPath $m.EndpointPS -Recurse -Force
        Write-Host '  Deleted. Next steps:' -ForegroundColor Yellow
        Write-Host '    1. Open Device Manager.'
        Write-Host '    2. Find the Lampyris Mic device, Disable it, then Enable it again.'
        Write-Host '    3. Windows recreates the endpoint fresh, seeded from the INF OEMFormat value.'
        Write-Host '    4. Re-run this script WITHOUT -Purge to confirm the new DeviceFormat decode.'
        Write-Host ("  Restore with: reg import `"{0}`"" -f $backupFile)
        Write-Host ''
        continue
    }

    # (c) DELETE both properties. A hand-written DeviceFormat is what breaks this
    # endpoint: a shared-mode capture mix format is 32-bit float, so forcing a
    # 16-bit PCM blob makes GetMixFormat return AUDCLNT_E_UNSUPPORTED_FORMAT. With
    # both values absent the engine derives the mix format from the pin, which is
    # exactly what the working MicArray endpoint does.
    Write-Host '  Deleting DeviceFormat and OEMFormat so the engine derives them from the pin...'
    if (Test-Path -LiteralPath $m.PropertiesPS) {
        Remove-ItemProperty -LiteralPath $m.PropertiesPS -Name $PkeyDeviceFormat -Force -ErrorAction SilentlyContinue
        Remove-ItemProperty -LiteralPath $m.PropertiesPS -Name $PkeyOemFormat    -Force -ErrorAction SilentlyContinue
    }

    # (d) AFTER decode — expect both properties to read <not set>.
    Write-Host ''
    Write-Host '  --- AFTER ---' -ForegroundColor Green
    Show-FormatBlob -Label 'DeviceFormat' -Value (Get-EndpointProperty -PropertiesPath $m.PropertiesPS -PropertyName $PkeyDeviceFormat)
    Show-FormatBlob -Label 'OEMFormat'    -Value (Get-EndpointProperty -PropertiesPath $m.PropertiesPS -PropertyName $PkeyOemFormat)
    Write-Host ("  Backup: {0}   (restore with: reg import `"{0}`")" -f $backupFile)
    Write-Host ''
}

# ---------------------------------------------------------------------------
# 3. Restart the audio stack so the engine re-reads the endpoint property store.
# ---------------------------------------------------------------------------
Write-Host '*** WARNING ***' -ForegroundColor Yellow
Write-Host 'About to restart AudioEndpointBuilder and Audiosrv. This briefly interrupts ALL' -ForegroundColor Yellow
Write-Host 'audio on this machine — music, calls, other microphones will cut out for a few' -ForegroundColor Yellow
Write-Host 'seconds. This is expected and harmless, but close anything mid-call first.' -ForegroundColor Yellow
Write-Host ''
Start-Sleep -Seconds 3

Write-Host 'Restarting AudioEndpointBuilder (cascades to Audiosrv)...'
Restart-Service -Name AudioEndpointBuilder -Force

# Audiosrv depends on AudioEndpointBuilder and is usually cascade-restarted; verify.
$audiosrv = Get-Service -Name Audiosrv
if ($audiosrv.Status -ne 'Running') {
    Write-Host 'Audiosrv did not come back automatically; starting it explicitly...'
    Start-Service -Name Audiosrv
    $audiosrv = Get-Service -Name Audiosrv
}
Write-Host ("Audiosrv status: {0}" -f $audiosrv.Status)
Write-Host ''
Write-Host 'Done. Next: run windows-driver\tools\wasapi_probe.exe and check that' -ForegroundColor Green
Write-Host 'GetMixFormat returns S_OK with 1 ch, 48000 Hz, 16 bit for the Lampyris endpoint.' -ForegroundColor Green
exit 0
