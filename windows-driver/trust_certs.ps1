$Cer = Join-Path $PSScriptRoot "lampyris-sysvad\TabletAudioSample\x64\SignedPackage\lampyris-mic.cer"
if (-not (Test-Path -LiteralPath $Cer)) {
    Write-Host "ERROR: certificate not found at: $Cer" -ForegroundColor Red
    Write-Host "       Run sign_driver.ps1 first to build the signed package." -ForegroundColor Red
    exit 1
}
bcdedit /set testsigning on
certutil -addstore -f root "$Cer"
certutil -addstore -f TrustedPublisher "$Cer"
