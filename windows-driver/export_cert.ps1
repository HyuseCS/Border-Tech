# Export the WDK test certificate from the local certificate store to a proper .cer file
$thumb = "DF5BB8BF921D9CFCF15636514913EFF63FFC257D"
$cert = Get-ChildItem Cert:\CurrentUser\My\$thumb
$outPath = "D:\Border-Tech\windows-driver\lampyris-sysvad\TabletAudioSample\x64\SignedPackage\lampyris-mic.cer"
Export-Certificate -Cert $cert -FilePath $outPath -Type CERT -Force
Write-Host "Exported certificate to: $outPath"
Write-Host "File size: $((Get-Item $outPath).Length) bytes"
Write-Host ""
# Verify it's valid
certutil $outPath
