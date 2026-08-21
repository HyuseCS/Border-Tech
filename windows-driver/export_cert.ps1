# Export the WDK test certificate from the local certificate store to a proper .cer file
$thumb = "CDDFCA2E7FC4670B8E00D7AF54347FFD0E5F4C3F"
$cert = Get-ChildItem Cert:\CurrentUser\My\$thumb
$outPath = (Join-Path $PSScriptRoot "lampyris-sysvad\TabletAudioSample\x64\SignedPackage\lampyris-mic.cer")
Export-Certificate -Cert $cert -FilePath $outPath -Type CERT -Force
Write-Host "Exported certificate to: $outPath"
Write-Host "File size: $((Get-Item $outPath).Length) bytes"
Write-Host ""
# Verify it's valid
certutil $outPath
