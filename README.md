# ThinkCharge

Lightweight **ThinkPad battery manager** for Windows and Linux — built with **Tauri 2**, **React**, and **Tailwind CSS**.

Set charge start/stop thresholds (like Lenovo Vantage), run off AC without holding the battery at 100%, and one-click **Charge to Full** when you need it.

## Features

- Charge thresholds via Lenovo `IBMPmDrv` (Windows) or `thinkpad_acpi` sysfs (Linux)
- System tray with settings, widget toggle, charge controls
- Transparent desktop widget (glass-style panel)
- Small native binary (~5–15 MB when built)

## Prerequisites

### Windows (dev + runtime)

- WebView2 (preinstalled on Windows 10/11)
- Lenovo Power and Battery drivers (`IBMPmDrv`) for threshold control
- **Administrator privileges** recommended — required to write Lenovo `PWRMGRV` registry keys and override Lenovo Vantage limits

### Linux

- `thinkpad_acpi` with charge threshold sysfs files
- Write access to battery sysfs (udev rules may be required)

### Development

- Node.js 20+ and npm
- Rust (stable) via rustup
- Visual Studio C++ Build Tools (Windows)

## Quick start

**Windows (recommended — no PowerShell script policy needed):**

```cmd
cd C:\Users\Zaku-T14\Projects\ThinkCharge
run.cmd
```

Or directly:

```powershell
cd C:\Users\Zaku-T14\Projects\ThinkCharge
npm.cmd install
npm.cmd run tauri:dev
```

If you prefer `.\run.ps1`, allow local scripts once:

```powershell
Set-ExecutionPolicy -Scope CurrentUser RemoteSigned
```

Build a release installer:

```powershell
npm.cmd run tauri:build
```

## Usage

| Action | How |
|--------|-----|
| Open settings | Tray → **Open Settings** or widget **Settings** button |
| Set thresholds | Adjust sliders → **Save & Apply** |
| Charge to 100% | Tray or widget → **Charge to Full** |
| Restore limits | Tray or widget → **Restore Thresholds** |
| Toggle widget | Tray → **Toggle Desktop Widget** |

Config: `%APPDATA%\ThinkCharge\config.json` (Windows) or `~/.config/ThinkCharge/config.json` (Linux)

## Project layout

```
src/              React UI (widget + settings)
src-tauri/        Rust backend (battery control, tray, config)
legacy/python/    Original Python prototype
```

## License

MIT
