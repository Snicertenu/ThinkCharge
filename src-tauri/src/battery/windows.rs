use super::windows_pwrmgr::{
    broadcast_settings_change, ensure_ibmpmdrv_service, read_registry_thresholds,
    write_charge_to_full_registry, write_release_registry, write_threshold_registry,
};
use super::{BatteryBackend, BatteryStatus, ThresholdState, read_battery_status};

use std::ffi::c_void;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::IO::DeviceIoControl;

const IOCTL_SELECT_MODE: u32 = 0x22261C;
const IOCTL_SET_START: u32 = 0x222630;
const IOCTL_SET_STOP: u32 = 0x222638;
const THRESHOLD_MODE_BATTERY1: u32 = 0x00000101;
const AUTOMATIC_MODE: u32 = 0x00000000;
const BATTERY1_PREFIX: u32 = 0x00000100;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn try_open_device() -> Result<HANDLE, String> {
    unsafe {
        let handle = CreateFileW(
            PCWSTR(wide(r"\\.\IBMPmDrv").as_ptr()),
            (FILE_GENERIC_READ | FILE_GENERIC_WRITE).0,
            Default::default(),
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
        .map_err(|_| "Cannot open IBMPmDrv device.".to_string())?;

        if handle == INVALID_HANDLE_VALUE {
            return Err("Failed to open IBMPmDrv device.".into());
        }

        Ok(handle)
    }
}

fn open_device() -> Result<HANDLE, String> {
    match try_open_device() {
        Ok(handle) => Ok(handle),
        Err(_) => {
            ensure_ibmpmdrv_service()?;
            std::thread::sleep(std::time::Duration::from_millis(500));
            try_open_device().map_err(|_| {
                "Lenovo Power Management driver (IBMPmDrv) not available. Run as Administrator.".into()
            })
        }
    }
}

fn device_ioctl(handle: HANDLE, ioctl: u32, value: u32) -> Result<(), String> {
    let mut input = value;
    let mut bytes_returned = 0u32;

    unsafe {
        DeviceIoControl(
            handle,
            ioctl,
            Some(&mut input as *mut _ as *mut c_void),
            std::mem::size_of::<u32>() as u32,
            None,
            0,
            Some(&mut bytes_returned),
            None,
        )
        .map_err(|e| format!("DeviceIoControl failed: {e}"))?;
    }

    Ok(())
}

fn with_device<F: FnOnce(HANDLE) -> Result<(), String>>(f: F) -> Result<(), String> {
    let handle = open_device()?;
    let result = f(handle);
    unsafe {
        let _ = CloseHandle(handle);
    }
    result
}

fn apply_thresholds_ioctl(handle: HANDLE, start: u8, stop: u8) -> Result<(), String> {
    let stop = stop.min(99);
    device_ioctl(handle, IOCTL_SELECT_MODE, THRESHOLD_MODE_BATTERY1)?;
    // Lenovo driver expects stop before start.
    device_ioctl(handle, IOCTL_SET_STOP, BATTERY1_PREFIX | stop as u32)?;
    device_ioctl(handle, IOCTL_SET_START, BATTERY1_PREFIX | start as u32)?;
    Ok(())
}

fn apply_charge_to_full_ioctl(handle: HANDLE) -> Result<(), String> {
    device_ioctl(handle, IOCTL_SELECT_MODE, THRESHOLD_MODE_BATTERY1)?;
    // No stop ceiling.
    device_ioctl(handle, IOCTL_SET_STOP, BATTERY1_PREFIX)?;
    // Resume charging whenever below 99% (driver maximum).
    device_ioctl(handle, IOCTL_SET_START, BATTERY1_PREFIX | 99)?;
    Ok(())
}

fn release_charge_limit_ioctl(handle: HANDLE) -> Result<(), String> {
    device_ioctl(handle, IOCTL_SELECT_MODE, THRESHOLD_MODE_BATTERY1)?;
    device_ioctl(handle, IOCTL_SET_STOP, BATTERY1_PREFIX)?;
    device_ioctl(handle, IOCTL_SET_START, BATTERY1_PREFIX)?;
    device_ioctl(handle, IOCTL_SELECT_MODE, AUTOMATIC_MODE)?;
    Ok(())
}

pub struct WindowsThinkPadBackend {
    saved_start: Option<u8>,
    saved_stop: Option<u8>,
    full_charge_mode: bool,
}

impl WindowsThinkPadBackend {
    pub fn new() -> Self {
        let (start, stop, enabled) = read_registry_thresholds().unwrap_or((None, None, false));
        Self {
            saved_start: if enabled { start } else { None },
            saved_stop: if enabled { stop } else { None },
            full_charge_mode: false,
        }
    }

    fn driver_available(&self) -> bool {
        try_open_device().is_ok()
    }
}

impl BatteryBackend for WindowsThinkPadBackend {
    fn get_status(&self) -> BatteryStatus {
        read_battery_status()
    }

    fn get_thresholds(&self) -> ThresholdState {
        if !self.driver_available() {
            return ThresholdState {
                supported: false,
                enabled: false,
                start: None,
                stop: None,
                backend: "windows-ibmpmdrv".into(),
                message: "Lenovo PM driver not available. Try running as Administrator.".into(),
            };
        }

        let (reg_start, reg_stop, reg_enabled) =
            read_registry_thresholds().unwrap_or((None, None, false));

        ThresholdState {
            supported: true,
            enabled: (self.saved_start.is_some() || reg_enabled) && !self.full_charge_mode,
            start: self.saved_start.or(reg_start),
            stop: self.saved_stop.or(reg_stop),
            backend: "windows-ibmpmdrv".into(),
            message: "Using Lenovo IBMPmDrv + PWRMGRV (overrides Vantage limits).".into(),
        }
    }

    fn set_thresholds(&mut self, start: u8, stop: u8) -> Result<(), String> {
        write_threshold_registry(start, stop)?;
        with_device(|handle| apply_thresholds_ioctl(handle, start, stop))?;
        broadcast_settings_change();

        self.saved_start = Some(start);
        self.saved_stop = Some(stop);
        self.full_charge_mode = false;
        Ok(())
    }

    fn charge_to_full(&mut self) -> Result<(), String> {
        // 1. Clear any latched stop from the previous threshold pair.
        with_device(release_charge_limit_ioctl)?;
        // 2. Charge whenever below 99%, with no stop ceiling (top-off uses automatic mode).
        write_charge_to_full_registry()?;
        with_device(apply_charge_to_full_ioctl)?;
        broadcast_settings_change();
        self.full_charge_mode = true;
        Ok(())
    }

    fn charge_to_full_top_off(&mut self) -> Result<(), String> {
        write_release_registry()?;
        with_device(release_charge_limit_ioctl)?;
        broadcast_settings_change();
        self.full_charge_mode = true;
        Ok(())
    }

    fn restore_thresholds(&mut self, start: u8, stop: u8) -> Result<(), String> {
        self.set_thresholds(start, stop)
    }
}