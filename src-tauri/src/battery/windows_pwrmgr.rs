//! Lenovo PWRMGRV registry + IBMPMDRV service helpers.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE};
use winreg::RegKey;

const DATA_PATH: &str = r"SOFTWARE\WOW6432Node\Lenovo\PWRMGRV\ConfKeys\Data";
const SERVICE_NAME: &str = "IBMPMDRV";

pub fn find_battery_subkey() -> Result<String, String> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let data = hklm
        .open_subkey(DATA_PATH)
        .map_err(|e| format!("PWRMGRV registry not found: {e}"))?;

    for name in data
        .enum_keys()
        .map(|r| r.map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?
    {
        let sub = data
            .open_subkey(&name)
            .map_err(|e| format!("Failed to open battery key {name}: {e}"))?;
        if sub.get_value::<u32, _>("ChargeStartPercentage").is_ok() {
            return Ok(name);
        }
    }

    Err("No Lenovo battery configuration key found in PWRMGRV registry.".into())
}

fn battery_key() -> Result<RegKey, String> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let name = find_battery_subkey()?;
    hklm.open_subkey_with_flags(format!("{DATA_PATH}\\{name}"), KEY_WRITE)
        .map_err(|e| format!("Cannot write battery registry (admin required?): {e}"))
}

pub fn read_registry_thresholds() -> Result<(Option<u8>, Option<u8>, bool), String> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let name = find_battery_subkey()?;
    let key = hklm
        .open_subkey_with_flags(format!("{DATA_PATH}\\{name}"), KEY_READ)
        .map_err(|e| e.to_string())?;

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
