# sign_driver.ps1 - Generate catalog file and test-sign the Lampyris driver package
# Run from: windows-driver\ directory
# Usage: powershell -ExecutionPolicy Bypass -File sign_driver.ps1

$ErrorActionPreference = "Stop"

# --- Configuration ---
$WdkBinRoot  = "C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0"
$Inf2Cat     = Join-Path $WdkBinRoot "x86\Inf2Cat.exe"
$SignTool    = Join-Path $WdkBinRoot "x64\signtool.exe"
$CertThumb   = "DF5BB8BF921D9CFCF15636514913EFF63FFC257D"
$DriverDir   = Join-Path $PSScriptRoot "lampyris-sysvad\TabletAudioSample\x64\Release"

# Staging directory: clean folder with only the files Inf2Cat needs
$StageDir    = Join-Path $PSScriptRoot "lampyris-sysvad\TabletAudioSample\x64\SignedPackage"

# Verify tools exist
if (-not (Test-Path $Inf2Cat))  { throw "Inf2Cat not found at: $Inf2Cat" }
if (-not (Test-Path $SignTool)) { throw "SignTool not found at: $SignTool" }
if (-not (Test-Path $DriverDir)){ throw "Driver release directory not found at: $DriverDir" }

Write-Host "============================================" -ForegroundColor Cyan
Write-Host " Lampyris Driver Signing Pipeline" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

# --- Step 0: Create a clean staging directory ---
# Inf2Cat processes ALL .inf files in a directory, so we stage only our driver's files.
Write-Host "[0/4] Preparing staging directory..." -ForegroundColor Yellow
if (Test-Path $StageDir) { Remove-Item $StageDir -Recurse -Force }
New-Item -ItemType Directory -Path $StageDir -Force | Out-Null

# Copy only the files needed for the driver package
Copy-Item (Join-Path $DriverDir "lampyris-mic.sys") $StageDir
Copy-Item (Join-Path $DriverDir "ComponentizedAudioSample.inf") $StageDir
Copy-Item (Join-Path $DriverDir "lampyris-mic.cer") $StageDir
Write-Host "  -> Staged: lampyris-mic.sys, ComponentizedAudioSample.inf, lampyris-mic.cer" -ForegroundColor Green
Write-Host ""

# --- Step 1: Generate the catalog file ---
Write-Host "[1/4] Generating catalog file (sysvad.cat) with Inf2Cat..." -ForegroundColor Yellow
& $Inf2Cat /driver:"$StageDir" /os:10_x64 /verbose
if ($LASTEXITCODE -ne 0) {
    throw "Inf2Cat failed with exit code $LASTEXITCODE"
}

$CatFile = Join-Path $StageDir "sysvad.cat"
if (-not (Test-Path $CatFile)) {
    throw "Inf2Cat ran but sysvad.cat was not created. Check the INF CatalogFile= directive."
}
Write-Host "  -> sysvad.cat created successfully." -ForegroundColor Green
Write-Host ""

# --- Step 2: Sign the catalog file ---
Write-Host "[2/4] Signing sysvad.cat with test certificate..." -ForegroundColor Yellow
& $SignTool sign /v /s My /sha1 $CertThumb /fd sha256 /t http://timestamp.digicert.com "$CatFile"
if ($LASTEXITCODE -ne 0) {
    Write-Host "  -> Timestamping server unreachable, signing without timestamp..." -ForegroundColor DarkYellow
    & $SignTool sign /v /s My /sha1 $CertThumb /fd sha256 "$CatFile"
    if ($LASTEXITCODE -ne 0) {
        throw "SignTool failed to sign sysvad.cat"
    }
}
Write-Host "  -> sysvad.cat signed successfully." -ForegroundColor Green
Write-Host ""

# --- Step 3: Sign the .sys driver binary ---
$SysFile = Join-Path $StageDir "lampyris-mic.sys"
Write-Host "[3/4] Signing lampyris-mic.sys with test certificate..." -ForegroundColor Yellow
& $SignTool sign /v /s My /sha1 $CertThumb /fd sha256 /t http://timestamp.digicert.com "$SysFile"
if ($LASTEXITCODE -ne 0) {
    Write-Host "  -> Timestamping server unreachable, signing without timestamp..." -ForegroundColor DarkYellow
    & $SignTool sign /v /s My /sha1 $CertThumb /fd sha256 "$SysFile"
    if ($LASTEXITCODE -ne 0) {
        throw "SignTool failed to sign lampyris-mic.sys"
    }
}
Write-Host "  -> lampyris-mic.sys signed successfully." -ForegroundColor Green
Write-Host ""

# --- Step 4: Verification ---
Write-Host "[4/4] Verifying signatures..." -ForegroundColor Yellow
Write-Host ""
Write-Host "--- sysvad.cat ---" -ForegroundColor White
& $SignTool verify /v /pa "$CatFile"
Write-Host ""
Write-Host "--- lampyris-mic.sys ---" -ForegroundColor White
& $SignTool verify /v /pa "$SysFile"
Write-Host ""

Write-Host "============================================" -ForegroundColor Green
Write-Host " Done! Signed driver package ready." -ForegroundColor Green
Write-Host "============================================" -ForegroundColor Green
Write-Host ""
Write-Host "Signed package location:" -ForegroundColor White
Write-Host "  $StageDir" -ForegroundColor White
Write-Host ""
Write-Host "Files to copy to the VM:" -ForegroundColor White
Write-Host "  - lampyris-mic.sys  (signed driver)" -ForegroundColor White
Write-Host "  - ComponentizedAudioSample.inf  (install config)" -ForegroundColor White
Write-Host "  - sysvad.cat  (signed catalog)" -ForegroundColor White
Write-Host "  - lampyris-mic.cer  (test certificate)" -ForegroundColor White
