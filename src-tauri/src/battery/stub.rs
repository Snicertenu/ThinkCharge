use super::{BatteryBackend, BatteryStatus, ThresholdState, read_battery_status};

pub struct StubBackend;

impl BatteryBackend for StubBackend {
    fn get_status(&self) -> BatteryStatus {
        read_battery_status()
    }

    fn get_thresholds(&self) -> ThresholdState {
        ThresholdState {
            supported: false,
            enabled: false,
            start: None,
            stop: None,
            backend: "stub".into(),
            message: "Battery threshold control is not supported on this platform.".into(),
        }
    }

    fn set_thresholds(&mut self, _start: u8, _stop: u8) -> Result<(), String> {
        Err("Unsupported platform".into())
    }

    fn charge_to_full(&mut self) -> Result<(), String> {
        Err("Unsupported platform".into())
    }

    fn restore_thresholds(&mut self, _start: u8, _stop: u8) -> Result<(), String> {
        Err("Unsupported platform".into())
    }
}
