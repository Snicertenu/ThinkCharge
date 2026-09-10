@echo off
setlocal
cd /d "%~dp0"

REM Prefer Node/Cargo already on PATH; fall back to common install locations.
where node >nul 2>&1
if errorlevel 1 if exist "%ProgramFiles%\nodejs\node.exe" set "PATH=%ProgramFiles%\nodejs;%PATH%"
if exist "%USERPROFILE%\.cargo\bin" set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

where node >nul 2>&1
if errorlevel 1 (
  echo Node.js was not found. Install Node 20+ from https://nodejs.org/ then retry.
  exit /b 1
)

npm.cmd run tauri:dev
