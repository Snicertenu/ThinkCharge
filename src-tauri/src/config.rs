use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub start_threshold: u8,
    pub stop_threshold: u8,
    pub thresholds_enabled: bool,
    pub show_desktop_widget: bool,
    pub widget_x: i32,
    pub widget_y: i32,
    pub widget_opacity: f64,
    pub widget_scale: f64,
    #[serde(default)]
    pub charge_to_full_active: bool,
    #[serde(default)]
    pub charge_to_full_armed: bool,
    #[serde(default)]
    pub charge_to_full_complete: bool,
    #[serde(default)]
    pub charge_to_full_verifying: bool,
    pub show_battery_percent: bool,
    pub show_power_source: bool,
    pub show_charging_status: bool,
    #[serde(default = "default_use_status_icons")]
    pub use_status_icons: bool,
    pub match_os_theme: bool,
}

fn default_use_status_icons() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            start_threshold: 75,
            stop_threshold: 80,
            thresholds_enabled: true,
            show_desktop_widget: true,
            widget_x: 40,
            widget_y: 40,
            widget_opacity: 0.92,
            widget_scale: 1.05,
            charge_to_full_active: false,
            charge_to_full_armed: false,
            charge_to_full_complete: false,
            charge_to_full_verifying: false,
            show_battery_percent: true,
            show_power_source: true,
            show_charging_status: true,
            use_status_icons: true,
            match_os_theme: true,
        }
    }
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.start_threshold > 99 {
            return Err("Start threshold must be between 0 and 99".into());
        }
        if self.stop_threshold < 1 || self.stop_threshold > 100 {
            return Err("Stop threshold must be between 1 and 100".into());
        }
        if self.stop_threshold.saturating_sub(self.start_threshold) < 4 {
            return Err("Stop threshold must be at least 4% above start threshold".into());
        }
        if self.widget_scale < 0.85 || self.widget_scale > 1.75 {
            return Err("Widget scale must be between 0.85 and 1.75".into());
        }
        Ok(())
    }
}

pub fn config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("ThinkCharge").join("config.json")
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    if !path.exists() {
        let cfg = AppConfig::default();
        let _ = save_config(&cfg);
        return cfg;
    }

    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<AppConfig>(&raw).ok())
        .map(|mut cfg| {
            // Charge-to-full is session-only; never restore from disk.
            cfg.charge_to_full_active = false;
            cfg.charge_to_full_armed = false;
            cfg.charge_to_full_complete = false;
            cfg.charge_to_full_verifying = false;
            cfg
        })
        .unwrap_or_default()
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut to_save = config.clone();
    to_save.charge_to_full_active = false;
    to_save.charge_to_full_armed = false;
    to_save.charge_to_full_complete = false;
    to_save.charge_to_full_verifying = false;
    let raw = serde_json::to_string_pretty(&to_save).map_err(|e| e.to_string())?;
    std::fs::write(path, raw).map_err(|e| e.to_string())
}

pub const WIDGET_BASE_WIDTH: f64 = 248.0;
pub const WIDGET_BASE_HEIGHT: f64 = 168.0;
pub const WIDGET_CHARGE_BAR_EXTRA: f64 = 28.0;

pub fn widget_size(scale: f64, show_charge_bar: bool) -> (u32, u32) {
    let height = WIDGET_BASE_HEIGHT + if show_charge_bar { WIDGET_CHARGE_BAR_EXTRA } else { 0.0 };
    (
        (WIDGET_BASE_WIDTH * scale).round() as u32,
        (height * scale).round() as u32,
    )
}
