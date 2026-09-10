//! Lenovo PWRMGRV registry + IBMPMDRV service helpers.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::OnceLock;

use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE};
use winreg::RegKey;

/// Common Lenovo Power Manager registry roots across 32/64-bit and OEM layouts.
const DATA_PATH_CANDIDATES: &[&str] = &[
    r"SOFTWARE\WOW6432Node\Lenovo\PWRMGRV\ConfKeys\Data",
    r"SOFTWARE\Lenovo\PWRMGRV\ConfKeys\Data",
];

const SERVICE_NAME: &str = "IBMPMDRV";

#[derive(Clone)]
struct PwrmgrLocation {
    data_path: String,
    battery_key: String,
}

static PWRMGR_LOCATION: OnceLock<Result<PwrmgrLocation, String>> = OnceLock::new();

fn discover_pwrmgr_location() -> Result<PwrmgrLocation, String> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let mut last_err = String::from("PWRMGRV registry not found");

    for data_path in DATA_PATH_CANDIDATES {
        let data = match hklm.open_subkey(data_path) {
            Ok(key) => key,
            Err(e) => {
                last_err = format!("{data_path}: {e}");
                continue;
            }
        };

        let names = match data
            .enum_keys()
            .map(|r| r.map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(names) => names,
            Err(e) => {
                last_err = format!("{data_path}: {e}");
                continue;
            }
        };

        for name in names {
            let sub = match data.open_subkey(&name) {
                Ok(key) => key,
                Err(_) => continue,
            };
            if sub.get_value::<u32, _>("ChargeStartPercentage").is_ok()
                || sub.get_value::<u32, _>("ChargeStopPercentage").is_ok()
            {
                return Ok(PwrmgrLocation {
                    data_path: (*data_path).to_string(),
                    battery_key: name,
                });
            }
        }

        last_err = format!("No battery key under {data_path}");
    }

    Err(format!(
        "No Lenovo battery configuration key found in PWRMGRV registry ({last_err}). \
         Install Lenovo Power Manager / Vantage battery drivers."
    ))
}

fn pwrmgr_location() -> Result<&'static PwrmgrLocation, String> {
    let cached = PWRMGR_LOCATION.get_or_init(discover_pwrmgr_location);
    cached.as_ref().map_err(|e| e.clone())
}

fn battery_key() -> Result<RegKey, String> {
    let loc = pwrmgr_location()?;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    hklm.open_subkey_with_flags(
        format!("{}\\{}", loc.data_path, loc.battery_key),
        KEY_WRITE,
    )
    .map_err(|e| format!("Cannot write battery registry (admin required?): {e}"))
}

fn battery_key_read() -> Result<RegKey, String> {
    let loc = pwrmgr_location()?;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    hklm.open_subkey_with_flags(format!("{}\\{}", loc.data_path, loc.battery_key), KEY_READ)
        .map_err(|e| e.to_string())
}

pub fn read_registry_thresholds() -> Result<(Option<u8>, Option<u8>, bool), String> {
    let key = battery_key_read()?;

    let start: u32 = key.get_value("ChargeStartPercentage").unwrap_or(0);
    let stop: u32 = key.get_value("ChargeStopPercentage").unwrap_or(100);
    let start_ctrl: u32 = key.get_value("ChargeStartControl").unwrap_or(0);
    let stop_ctrl: u32 = key.get_value("ChargeStopControl").unwrap_or(0);
    let enabled = start_ctrl != 0 && stop_ctrl != 0;

    Ok((
        Some(start.min(99) as u8),
        Some(stop.min(100) as u8),
        enabled,
    ))
}

pub fn write_threshold_registry(start: u8, stop: u8) -> Result<(), String> {
    let key = battery_key()?;
    key.set_value("ChargeStartPercentage", &(start as u32))
        .map_err(|e| e.to_string())?;
    key.set_value("ChargeStopPercentage", &(stop as u32))
        .map_err(|e| e.to_string())?;
    key.set_value("ChargeStartControl", &1u32)
        .map_err(|e| e.to_string())?;
    key.set_value("ChargeStopControl", &1u32)
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn write_charge_to_full_registry() -> Result<(), String> {
    let key = battery_key()?;
    // Resume charging below 99% (driver max). Stop control off = no ceiling.
    key.set_value("ChargeStartPercentage", &99u32)
        .map_err(|e| e.to_string())?;
    key.set_value("ChargeStopPercentage", &100u32)
        .map_err(|e| e.to_string())?;
    key.set_value("ChargeStartControl", &1u32)
        .map_err(|e| e.to_string())?;
    key.set_value("ChargeStopControl", &0u32)
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn write_release_registry() -> Result<(), String> {
    let key = battery_key()?;
    key.set_value("ChargeStartControl", &0u32)
        .map_err(|e| e.to_string())?;
    key.set_value("ChargeStopControl", &0u32)
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn ensure_ibmpmdrv_service() -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Services::{
        CloseServiceHandle, OpenSCManagerW, OpenServiceW, StartServiceW, SC_MANAGER_CONNECT,
        SERVICE_START,
    };

    unsafe {
        let scm = OpenSCManagerW(None, None, SC_MANAGER_CONNECT)
            .map_err(|e| format!("OpenSCManager failed: {e}"))?;

        let service_name: Vec<u16> = OsStr::new(SERVICE_NAME)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let service = OpenServiceW(scm, PCWSTR(service_name.as_ptr()), SERVICE_START).map_err(
            |e| {
                let _ = CloseServiceHandle(scm);
                format!("Lenovo IBMPMDRV service not found: {e}")
            },
        )?;

        let _ = StartServiceW(service, None);
        let _ = CloseServiceHandle(service);
        let _ = CloseServiceHandle(scm);
    }

    Ok(())
}

pub fn broadcast_settings_change() {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
    };

    let section: Vec<u16> = OsStr::new("Policy")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut result = usize::default();
        let _ = SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(section.as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            1000,
            Some(&mut result),
        );
    }
}
