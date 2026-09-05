use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use starship_battery::units::electric_potential::volt;
use starship_battery::units::energy::watt_hour;
use starship_battery::units::ratio::percent as percent_unit;
use starship_battery::units::thermodynamic_temperature::degree_celsius;
use starship_battery::units::time::minute;
use starship_battery::{Manager, State, Technology};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryReport {
    pub generated_at_unix: u64,
    pub platform: String,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub technology: String,
    pub state: String,
    pub state_of_charge_percent: f64,
    pub state_of_health_percent: Option<f64>,
    pub health_rating: String,
    pub health_summary: String,
    pub energy_full_wh: Option<f64>,
    pub energy_design_wh: Option<f64>,
    pub voltage_v: Option<f64>,
    pub temperature_c: Option<f64>,
    pub cycle_count: Option<u32>,
    pub is_charging: bool,
    pub is_plugged: bool,
    pub time_to_empty_min: Option<u64>,
    pub time_to_full_min: Option<u64>,
    pub replacement_hint: Option<String>,
}

pub fn generate_battery_report() -> Result<BatteryReport, String> {
    let manager = Manager::new().map_err(|e| format!("Battery manager unavailable: {e}"))?;
    let mut batteries = manager
        .batteries()
        .map_err(|e| format!("Cannot enumerate batteries: {e}"))?;

    let battery = batteries
        .find_map(|b| b.ok())
        .ok_or_else(|| "No battery found on this system.".to_string())?;

    let soc = battery.state_of_charge().get::<percent_unit>() as f64;
    let soh_ratio = battery.state_of_health().get::<percent_unit>() as f64;
    let soh = if soh_ratio > 0.0 && soh_ratio <= 100.0 {
        Some(soh_ratio)
    } else {
        None
    };

    let energy_full = battery.energy_full().get::<watt_hour>() as f64;
    let energy_design = battery.energy_full_design().get::<watt_hour>() as f64;
    let computed_soh = soh.or_else(|| {
        if energy_design > 0.0 {
            Some((energy_full / energy_design * 100.0).clamp(0.0, 100.0))
        } else {
            None
        }
    });

    let (health_rating, health_summary) = health_assessment(computed_soh, energy_full, energy_design);

    let state = battery.state();
    let is_charging = matches!(state, State::Charging);
    let is_plugged = matches!(state, State::Charging | State::Full);

    let vendor = battery.vendor().map(str::to_string);
    let model = battery.model().map(str::to_string);
    let replacement_hint = build_replacement_hint(vendor.as_deref(), model.as_deref());

    Ok(BatteryReport {
        generated_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        platform: std::env::consts::OS.to_string(),
        vendor,
        model,
        serial_number: battery.serial_number().map(str::to_string),
        technology: format_technology(battery.technology()),
        state: format_state(state),
        state_of_charge_percent: soc,
        state_of_health_percent: computed_soh,
        health_rating,
        health_summary,
        energy_full_wh: if energy_full > 0.0 {
            Some(energy_full)
        } else {
            None
        },
        energy_design_wh: if energy_design > 0.0 {
            Some(energy_design)
        } else {
            None
        },
        voltage_v: {
            let v = battery.voltage().get::<volt>() as f64;
            if v > 0.0 { Some(v) } else { None }
        },
        temperature_c: battery
            .temperature()
            .map(|t| t.get::<degree_celsius>() as f64),
        cycle_count: battery.cycle_count(),
        is_charging,
        is_plugged,
        time_to_empty_min: battery
            .time_to_empty()
            .map(|t| t.get::<minute>().max(0.0) as u64),
        time_to_full_min: battery
            .time_to_full()
            .map(|t| t.get::<minute>().max(0.0) as u64),
        replacement_hint,
    })
}

fn health_assessment(
    soh: Option<f64>,
    energy_full: f64,
    energy_design: f64,
) -> (String, String) {
    let Some(soh) = soh else {
        return (
            "Unknown".into(),
            "Health data is not available from this system.".into(),
        );
    };

    let rating = if soh >= 80.0 {
        "Good"
    } else if soh >= 60.0 {
        "Fair"
    } else {
        "Poor"
    };

    let mut summary = format!("Battery health is {soh:.0}% — rated {rating}.");
    if energy_design > 0.0 {
        summary.push_str(&format!(
            " Current full capacity is {energy_full:.1} Wh of {energy_design:.1} Wh design."
        ));
    }

    (rating.to_string(), summary)
}

fn build_replacement_hint(vendor: Option<&str>, model: Option<&str>) -> Option<String> {
    match (vendor, model) {
        (Some(v), Some(m)) => Some(format!(
            "Search for a replacement using part number \"{m}\" (vendor: {v}). Lenovo/ThinkPad batteries often use FRU or model numbers like this."
        )),
        (_, Some(m)) => Some(format!(
            "Search for a replacement using part number \"{m}\"."
        )),
        _ => None,
    }
}

fn format_technology(tech: Technology) -> String {
    format!("{tech}")
}

fn format_state(state: State) -> String {
    match state {
        State::Unknown => "Unknown",
        State::Charging => "Charging",
        State::Discharging => "Discharging",
        State::Empty => "Empty",
        State::Full => "Full",
    }
    .to_string()
}
