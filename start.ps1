Start-Process `
    -FilePath "c:\Dev\Windows-Recall\recall-cli\target\debug\recall-cli.exe" `
    -ArgumentList "start", "--interval", "30" `
    -WindowStyle Hidden