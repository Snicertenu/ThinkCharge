# ThinkCharge

Lightweight **ThinkPad battery manager** for Windows and Linux — built with **Tauri 2**, **React**, and **Tailwind CSS**.

Set charge start/stop thresholds (like Lenovo Vantage), run on AC without holding the battery at 100%, and one-click **Charge to Full** when you need a full cycle.

## Install on another ThinkPad (no command line)

### Windows

1. Build or download the installer:
   - After `npm run tauri:build`, use  
     `src-tauri/target/release/bundle/nsis/ThinkCharge_*_x64-setup.exe`
2. Copy that `.exe` to the other laptop (USB, network share, etc.).
3. Double-click the setup file and finish the wizard.
4. Launch **ThinkCharge** from the Start Menu or the desktop shortcut.

Windows will show a **UAC (Administrator)** prompt when ThinkCharge starts. Accept it so threshold writes can talk to Lenovo’s `IBMPmDrv` driver and Power Manager registry. No terminal is required.

**Requirements on the target laptop:**

- Windows 10/11 ThinkPad with Lenovo Power / Battery drivers (`IBMPmDrv` service)
- WebView2 (usually already installed; the setup can download it if missing)

You do **not** need Node.js, Rust, or this source tree on other laptops — only the installer.

### Linux

Build a package with `npm run tauri:build`, then install the generated `.deb` / AppImage from `src-tauri/target/release/bundle/`. Configure [udev rules](#linux-udev-rules) so threshold writes work without `sudo` for daily use.

## Features

- Charge thresholds via Lenovo `IBMPmDrv` (Windows) or `thinkpad_acpi` sysfs (Linux)
- Works across ThinkPad models: probes common PWRMGRV registry paths and both battery slots
- System tray with settings, widget toggle, and charge controls
- Transparent desktop widget (glass-style panel)
- Native installer with Start Menu + desktop shortcuts

## Prerequisites

### Windows (runtime)

- WebView2 (preinstalled on most Windows 10/11 PCs)
- Lenovo Power and Battery drivers (`IBMPmDrv`) for threshold control
- Administrator consent at launch (embedded in the app; see [Privilege boundaries](#privilege-boundaries--elevation))

### Linux (runtime)

- ThinkPad with `thinkpad_acpi` exposing charge threshold sysfs files:
  - `/sys/class/power_supply/BAT*/charge_control_start_threshold`
  - `/sys/class/power_supply/BAT*/charge_control_end_threshold`
- Write access to those files (udev rule recommended — see below)

### Development

| Requirement | Windows | Linux |
|-------------|---------|-------|
| Node.js 20+ and npm | Installer / package manager | Distro packages or nvm |
| Rust (stable) | [rustup](https://rustup.rs/) | [rustup](https://rustup.rs/) |
| Native toolchain | Visual Studio C++ Build Tools | `build-essential` (Debian/Ubuntu) or equivalent |
| WebView | WebView2 | WebKitGTK (e.g. `libwebkit2gtk-4.1-dev` and related Tauri deps) |

## Quick start (developers)

Clone the repo and open a terminal in the project root (the directory that contains `package.json`).

### Install dependencies

**Windows (cmd / PowerShell):**

```bat
npm.cmd install
```

**Linux:**

```bash
npm install
```

### Run in development

**Windows — recommended helper** (avoids PowerShell execution-policy issues):

```bat
run.cmd
```

Or directly:

```bat
npm.cmd run tauri:dev
```

If you use `.\run.ps1`, allow local scripts once:

```powershell
Set-ExecutionPolicy -Scope CurrentUser RemoteSigned
```

**Linux:**

```bash
npm run tauri:dev
```

> **Note:** The release `.exe` requests Administrator via its manifest. For `tauri:dev`, launch an elevated terminal when you need live threshold writes.

### Build a release installer

**Windows:**

```bat
npm.cmd run tauri:build
```

**Linux:**

```bash
npm run tauri:build
```

Artifacts:

| Platform | Output |
|----------|--------|
| Windows NSIS (recommended) | `src-tauri/target/release/bundle/nsis/ThinkCharge_*_x64-setup.exe` |
| Windows MSI | `src-tauri/target/release/bundle/msi/ThinkCharge_*_x64_*.msi` |
| Linux | `src-tauri/target/release/bundle/` (deb / AppImage / rpm as available) |

Share the **setup.exe** with other users — they never need the command line.

## Linux udev rules

By default, only root can write ThinkPad charge thresholds under sysfs. Grant a group write access so ThinkCharge can run as a normal user.

1. Create `/etc/udev/rules.d/99-thinkcharge-battery.rules`:

```udev
# Allow members of group "plugdev" to set ThinkPad charge thresholds
ACTION=="add|change", SUBSYSTEM=="power_supply", KERNEL=="BAT*", \
  ATTR{type}=="Battery", \
  TEST=="charge_control_start_threshold", \
  MODE="0664", GROUP="plugdev"

ACTION=="add|change", SUBSYSTEM=="power_supply", KERNEL=="BAT*", \
  ATTR{type}=="Battery", \
  TEST=="charge_control_end_threshold", \
  MODE="0664", GROUP="plugdev"
```

If your distribution does not use `plugdev`, replace it with another group (for example your login group, or a dedicated `thinkcharge` group).

2. Add your user to that group:

```bash
sudo usermod -aG plugdev "$USER"
```

3. Reload rules and reattach the battery device (or reboot):

```bash
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=power_supply
```

4. Log out and back in (or reboot) so group membership applies, then verify:

```bash
ls -l /sys/class/power_supply/BAT*/charge_control_*_threshold
# Expect mode 0664 and group plugdev (or the group you chose)

echo 80 | tee /sys/class/power_supply/BAT0/charge_control_end_threshold
# Should succeed without sudo once permissions are correct
```

**Alternative (not recommended for daily use):** run ThinkCharge with `sudo`. Prefer the udev approach so the UI and tray stay in your normal session.

## Privilege boundaries / elevation

| Platform | Protected resource | What needs elevation / access | Typical approach |
|----------|--------------------|-------------------------------|------------------|
| Windows | `\\.\IBMPmDrv` (IOCTL) | Open device + set start/stop / charge-to-full | App requests Administrator (UAC) on launch |
| Windows | `PWRMGRV` registry (Lenovo Power Manager) | Override / sync limits that Vantage also uses | Same elevated process |
| Linux | `charge_control_start_threshold` / `charge_control_end_threshold` | Write new percentage values | udev group write **or** root/`sudo` |

**Practical notes:**

- **Windows:** The packaged `ThinkCharge.exe` embeds `requireAdministrator`, so Start Menu / desktop launch shows UAC — no manual “Run as administrator” step.
- **Linux:** Reading thresholds is usually fine for any user; writing requires the udev rule (or root). Do not run the whole desktop session as root — fix sysfs permissions instead.
- **Least privilege:** Only the battery backends need elevated access. Config lives in the user profile and does not require admin/root to *exist*.
- **Charge to Full** temporarily widens limits (Windows IOCTL / Linux start+end → 100), then restores your saved thresholds when the session completes or is cancelled.
- **Other ThinkPads:** Registry lookup tries both `SOFTWARE\WOW6432Node\Lenovo\PWRMGRV\…` and `SOFTWARE\Lenovo\PWRMGRV\…`. IOCTLs are applied to battery slot 1 and 2 when present.

## Usage

| Action | How |
|--------|-----|
| Open settings | Tray → **Open Settings** or widget gear icon |
| Set thresholds | Adjust sliders → **Save & Apply** |
| Charge to 100% | Tray or widget → **Charge to Full** |
| Restore limits | Tray → **Restore Thresholds** |
| Toggle widget | Tray → **Toggle Desktop Widget** |

### Config location

| Platform | Path |
|----------|------|
| Windows | `%APPDATA%\ThinkCharge\config.json` |
| Linux | `~/.config/ThinkCharge\config.json` |

## Project layout

```
src/                 React UI (widget + settings)
src-tauri/           Rust backend (battery control, tray, config)
src-tauri/windows/   Windows app.manifest (UAC elevation)
run.cmd / run.ps1    Dev-only helpers for `tauri:dev`
```

## License

MIT
