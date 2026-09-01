# Launch ThinkCharge (Tauri dev mode)
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$env:Path = "C:\Program Files\nodejs;$env:USERPROFILE\.cargo\bin;" + $env:Path
Set-Location $Root
npm.cmd run tauri:dev
