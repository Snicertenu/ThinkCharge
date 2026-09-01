use std::fs;
use std::path::{Path, PathBuf};

use super::{BatteryBackend, BatteryStatus, ThresholdState, read_battery_status};

pub struct LinuxThinkPadBackend {
    battery_path: Option<PathBuf>,
    saved_start: Option<u8>,
    saved_stop: Option<u8>,
    full_charge_mode: bool,
}

impl LinuxThinkPadBackend {
    pub fn new() -> Self {
        Self {
            battery_path: Self::find_battery_path(),
            saved_start: None,
            saved_stop: None,
            full_charge_mode: false,
        }
    }

    fn find_battery_path() -> Option<PathBuf> {
        let root = Path::new("/sys/class/power_supply");
        let Ok(entries) = fs::read_dir(root) else {
            return None;
        };

        let mut paths: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("BAT"))
            })
            .filter(|p| {
                p.join("charge_control_start_threshold").exists()
                    && p.join("charge_control_end_threshold").exists()
            })
            .collect();

        paths.sort();
        paths.into_iter().next()
    }

    fn start_file(&self) -> Result<PathBuf, String> {
        self.battery_path
            .as_ref()
            .map(|p| p.join("charge_control_start_threshold"))
            .ok_or_else(|| "No ThinkPad battery sysfs interface found".into())
    }

    fn end_file(&self) -> Result<PathBuf, String> {
        self.battery_path
            .as_ref()
            .map(|p| p.join("charge_control_end_threshold"))
            .ok_or_else(|| "No ThinkPad battery sysfs interface found".into())
    }

    fn read_int(path: &Path) -> Option<u8> {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
    }

    fn write_int(path: &Path, value: u8) -> Result<(), String> {
        fs::write(path, value.to_string()).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                "Permission denied writing battery thresholds. Configure udev rules or run with privileges.".into()
            } else {
                format!("Failed to write {}: {e}", path.display())
            }
        })
    }
}

impl BatteryBackend for LinuxThinkPadBackend {
    fn get_status(&self) -> BatteryStatus {
        read_battery_status()
    }

    fn get_thresholds(&self) -> ThresholdState {
        if self.battery_path.is_none() {
            return ThresholdState {
                supported: false,
                enabled: false,
                start: None,
                stop: None,
                backend: "linux-thinkpad-acpi".into(),
                message: "thinkpad_acpi charge threshold sysfs not found.".into(),
            };
        }

        let start = self.start_file().ok().and_then(|p| Self::read_int(&p));
        let stop = self.end_file().ok().and_then(|p| Self::read_int(&p));
        let enabled = stop.is_some_and(|v| v < 100) && !self.full_charge_mode;

        ThresholdState {
            supported: true,
            enabled,
            start: start.or(self.saved_start),
            stop: stop.or(self.saved_stop),
            backend: "linux-thinkpad-acpi".into(),
            message: format!(
                "Using {} sysfs interface.",
                self.battery_path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("BAT0")
            ),
        }
    }

    fn set_thresholds(&mut self, start: u8, stop: u8) -> Result<(), String> {
        let end = stop.min(99);
        let end_file = self.end_file()?;
        let start_file = self.start_file()?;
        let current_end = Self::read_int(&end_file).unwrap_or(100);

        if end < current_end {
            Self::write_int(&end_file, end)?;
            Self::write_int(&start_file, start)?;
        } else {
            Self::write_int(&start_file, start)?;
            Self::write_int(&end_file, end)?;
        }

        self.saved_start = Some(start);
        self.saved_stop = Some(stop);
        self.full_charge_mode = false;
        Ok(())
    }

    fn charge_to_full(&mut self) -> Result<(), String> {
        let end_file = self.end_file()?;
        let start_file = self.start_file()?;
        // Start threshold = resume when below this level. Use 100 so any SOC < 100% charges.
        Self::write_int(&end_file, 100)?;
        Self::write_int(&start_file, 100)?;
        self.full_charge_mode = true;
        Ok(())
    }

    fn restore_thresholds(&mut self, start: u8, stop: u8) -> Result<(), String> {
        self.set_thresholds(start, stop)
    }
}
