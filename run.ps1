# Launch ThinkCharge (Tauri dev mode) — for developers only.
# End users should install ThinkCharge_*-setup.exe instead.

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $Root

if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
  $nodeDir = Join-Path ${env:ProgramFiles} "nodejs"
  if (Test-Path (Join-Path $nodeDir "node.exe")) {
    $env:Path = "$nodeDir;" + $env:Path
  }
}

$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if (Test-Path $cargoBin) {
  $env:Path = "$cargoBin;" + $env:Path
}

if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
  Write-Error "Node.js was not found. Install Node 20+ from https://nodejs.org/ then retry."
  exit 1
}

npm.cmd run tauri:dev
