Get-ChildItem Cert:\CurrentUser\My | Where-Object {$_.Subject -like '*WDKTestCert*'} | Format-List Subject,Thumbprint
