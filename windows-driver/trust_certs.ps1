bcdedit /set testsigning on
certutil -addstore -f root "D:\Border-Tech\windows-driver\lampyris-sysvad\TabletAudioSample\x64\SignedPackage\lampyris-mic.cer"
certutil -addstore -f TrustedPublisher "D:\Border-Tech\windows-driver\lampyris-sysvad\TabletAudioSample\x64\SignedPackage\lampyris-mic.cer"
