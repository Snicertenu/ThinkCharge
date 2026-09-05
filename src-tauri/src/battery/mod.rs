use serde::Serialize;
use starship_battery::units::ratio::percent as percent_unit;
use std::sync::Mutex;

static BATTERY_MANAGER: Mutex<Option<starship_battery::Manager>> = Mutex::new(None);

fn with_battery_manager<T>(mut f: impl FnMut(&starship_battery::Manager) -> Option<T>) -> Option<T> {
    let mut guard = match BATTERY_MANAGER.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    if guard.is_none() {
        *guard = starship_battery::Manager::new().ok();
    }
    let Some(manager) = guard.as_ref() else {
        return None;
    };
    match f(manager) {
        Some(value) => Some(value),
        None => {
            // Recreate on soft failure (e.g. transient WMI/COM glitch after long uptime).
            *guard = starship_battery::Manager::new().ok();
            guard.as_ref().and_then(|m| f(m))
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryStatus {
    pub percent: u8,
    pub percent_exact: f64,
    pub is_charging: bool,
    pub is_plugged: bool,
    pub is_full: bool,
    pub time_remaining_sec: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThresholdState {
    pub supported: bool,
    pub enabled: bool,
    pub start: Option<u8>,
    pub stop: Option<u8>,
    pub backend: String,
    pub message: String,
}

pub trait BatteryBackend: Send {
    fn get_status(&self) -> BatteryStatus;
    fn get_thresholds(&self) -> ThresholdState;
    fn set_thresholds(&mut self, start: u8, stop: u8) -> Result<(), String>;
    fn charge_to_full(&mut self) -> Result<(), String>;
    /// Switch to unrestricted charging for the final top-off to 100%.
    fn charge_to_full_top_off(&mut self) -> Result<(), String> {
        self.charge_to_full()
    }
    fn restore_thresholds(&mut self, start: u8, stop: u8) -> Result<(), String>;

    /// Re-apply the active policy to override external tools (e.g. Lenovo Vantage).
    /// Implementations should keep this cheap (no broadcasts / full registry rewrites).
    fn maintain_policy(
        &mut self,
        charge_to_full: bool,
        enabled: bool,
        start: u8,
        stop: u8,
    ) -> Result<(), String> {
        if charge_to_full {
            self.reassert_charge_to_full(true)
        } else if enabled {
            self.set_thresholds(start, stop)
        } else {
            Ok(())
        }
    }

    /// Quietly reassert charge-to-full IOCTL state (for background monitors).
    fn reassert_charge_to_full(&mut self, top_off: bool) -> Result<(), String> {
        if top_off {
            self.charge_to_full_top_off()
        } else {
            self.charge_to_full()
        }
    }
}

pub fn read_battery_status() -> BatteryStatus {
    let empty = BatteryStatus {
        percent: 0,
        percent_exact: 0.0,
        is_charging: false,
        is_plugged: false,
        is_full: false,
        time_remaining_sec: None,
    };

    with_battery_manager(|manager| {
        let Ok(mut batteries) = manager.batteries() else {
            return None;
        };
        let Some(Ok(battery)) = batteries.next() else {
            return Some(empty.clone());
        };

        let percent_exact = battery
            .state_of_charge()
            .get::<percent_unit>()
            .clamp(0.0, 100.0) as f64;
        // Round for the status line so we align with the Windows tray indicator.
        let level = percent_exact.round().clamp(0.0, 100.0) as u8;
        let state = battery.state();
        let is_full = matches!(state, starship_battery::State::Full);
        let is_charging = matches!(state, starship_battery::State::Charging);
        let is_plugged = matches!(
            state,
            starship_battery::State::Charging | starship_battery::State::Full
        );
        let time_remaining_sec = battery
            .time_to_empty()
            .map(|t| t.value.max(0.0) as u64)
            .filter(|_| matches!(state, starship_battery::State::Discharging));

        Some(BatteryStatus {
            percent: level,
            percent_exact,
            is_charging,
            is_plugged,
            is_full,
            time_remaining_sec,
        })
    })
    .unwrap_or(empty)
}

pub use report::{BatteryReport, generate_battery_report};

mod report;
#[cfg(windows)]
mod windows_pwrmgr;
#[cfg(windows)]
mod windows;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(not(any(windows, target_os = "linux")))]
mod stub;

pub fn create_backend() -> Box<dyn BatteryBackend> {
    #[cfg(windows)]
    {
        return Box::new(windows::WindowsThinkPadBackend::new());
    }
    #[cfg(target_os = "linux")]
    {
        return Box::new(linux::LinuxThinkPadBackend::new());
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        return Box::new(stub::StubBackend);
    }
}
