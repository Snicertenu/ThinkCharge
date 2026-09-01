@echo off
setlocal
cd /d "%~dp0"
set "PATH=C:\Program Files\nodejs;%USERPROFILE%\.cargo\bin;%PATH%"
npm.cmd run tauri:dev
