mod battery;
mod config;

use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use battery::{BatteryReport, BatteryStatus, ThresholdState, create_backend, generate_battery_report};
use config::{AppConfig, load_config, save_config, widget_size};
use tauri::{
    Emitter, LogicalSize, Manager, State, WebviewWindow, WindowEvent,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

struct AppState {
    config: Mutex<AppConfig>,
    backend: Mutex<Box<dyn battery::BatteryBackend>>,
    charge_monitor: Mutex<ChargeMonitor>,
}

#[derive(Default)]
struct ChargeMonitor {
    armed: bool,
    active: bool,
    verifying: bool,
    complete: bool,
    was_charging: bool,
}

fn is_battery_full(status: &BatteryStatus) -> bool {
    if !status.is_plugged {
        return false;
    }
    // Rounded 100% on AC counts as full even if the OS still reports trickle charging.
    if status.percent >= 100 {
        return true;
    }
    !status.is_charging
        && (status.percent_exact >= 99.9
            || (status.percent >= 99 && status.is_full)
            || status.percent_exact >= 99.5)
}

fn is_topping_off(status: &BatteryStatus) -> bool {
    status.percent_exact >= 98.5 || status.percent >= 99
}

fn should_cancel_on_unplug(monitor: &ChargeMonitor) -> bool {
    // "Armed" on battery means waiting for AC — do not cancel that.
    monitor.active || monitor.verifying || monitor.complete
}

fn apply_saved_thresholds(state: &AppState) -> Result<(), String> {
    let (enabled, start, stop) = {
        let cfg = state.config.lock().map_err(|e| e.to_string())?;
        (
            cfg.thresholds_enabled,
            cfg.start_threshold,
            cfg.stop_threshold,
        )
    };
    if !enabled {
        return Ok(());
    }
    let mut backend = state.backend.lock().map_err(|e| e.to_string())?;
    backend.restore_thresholds(start, stop)
}

fn apply_thresholds_only(state: &AppState) -> Result<(), String> {
    let (enabled, start, stop) = {
        let cfg = state.config.lock().map_err(|e| e.to_string())?;
        (
            cfg.thresholds_enabled,
            cfg.start_threshold,
            cfg.stop_threshold,
        )
    };

    if enabled {
        let mut backend = state.backend.lock().map_err(|e| e.to_string())?;
        backend.set_thresholds(start, stop)?;
    }

    Ok(())
}

fn restore_thresholds_internal(state: &AppState) -> Result<(), String> {
    apply_saved_thresholds(state)?;

    {
        let mut monitor = state.charge_monitor.lock().map_err(|e| e.to_string())?;
        monitor.armed = false;
        monitor.active = false;
        monitor.verifying = false;
        monitor.complete = false;
        monitor.was_charging = false;
    }

    Ok(())
}

fn finish_charge_to_full_session(state: &AppState) -> Result<(), String> {
    apply_saved_thresholds(state)?;

    {
        let mut monitor = state.charge_monitor.lock().map_err(|e| e.to_string())?;
        monitor.armed = false;
        monitor.active = false;
        monitor.verifying = false;
        monitor.complete = true;
        monitor.was_charging = false;
    }

    Ok(())
}

fn cancel_charge_to_full_session(state: &AppState) -> Result<(), String> {
    apply_saved_thresholds(state)?;
    clear_charge_to_full_state(state);
    Ok(())
}

fn clear_charge_to_full_state(state: &AppState) {
    if let Ok(mut monitor) = state.charge_monitor.lock() {
        monitor.armed = false;
        monitor.active = false;
        monitor.verifying = false;
        monitor.complete = false;
        monitor.was_charging = false;
    }
}

fn clear_complete_state(state: &AppState) {
    clear_charge_to_full_state(state);
}

fn merge_runtime_config(state: &AppState) -> Result<AppConfig, String> {
    let mut cfg = state.config.lock().map_err(|e| e.to_string())?.clone();
    let monitor = state.charge_monitor.lock().map_err(|e| e.to_string())?;
    cfg.charge_to_full_armed = monitor.armed;
    cfg.charge_to_full_active = monitor.active;
    cfg.charge_to_full_verifying = monitor.verifying;
    cfg.charge_to_full_complete = monitor.complete;
    Ok(cfg)
}

fn show_charge_bar(cfg: &AppConfig) -> bool {
    cfg.charge_to_full_active || cfg.charge_to_full_verifying || cfg.charge_to_full_complete
}

fn resize_widget_window(
    window: &WebviewWindow,
    scale: f64,
    show_charge_bar: bool,
) -> Result<(), String> {
    let (width, height) = widget_size(scale, show_charge_bar);
    window
        .set_size(LogicalSize::new(width as f64, height as f64))
        .map_err(|e| e.to_string())
}

fn apply_widget_window(state: &AppState, app: &tauri::AppHandle) -> Result<(), String> {
    let cfg = merge_runtime_config(state)?;
    if let Some(window) = app.get_webview_window("widget") {
        resize_widget_window(&window, cfg.widget_scale, show_charge_bar(&cfg))?;
    }
    Ok(())
}

#[cfg(windows)]
fn configure_widget_window(window: &WebviewWindow) {
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
    };

    let _ = window.set_shadow(false);
    if let Ok(hwnd) = window.hwnd() {
        let preference = DWMWCP_DONOTROUND.0 as u32;
        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &preference as *const _ as *const _,
                std::mem::size_of::<u32>() as u32,
            );
        }
    }
}

#[cfg(not(windows))]
fn configure_widget_window(_window: &WebviewWindow) {}

fn activate_charge_to_full(
    app: &tauri::AppHandle,
    state: &AppState,
    status: &BatteryStatus,
) -> Result<(), String> {
    if is_battery_full(status) {
        finish_charge_to_full_session(state)?;
        let _ = app.emit("charge-to-full-complete", ());
        let _ = apply_widget_window(state, app);
        return Ok(());
    }

    reapply_charge_to_full(state, status);
    {
        let mut monitor = state.charge_monitor.lock().map_err(|e| e.to_string())?;
        monitor.active = true;
        monitor.verifying = false;
        monitor.was_charging = false;
    }
    // Drop monitor before apply_widget_window — std::Mutex is not reentrant and
    // merge_runtime_config also locks charge_monitor (would deadlock the UI).
    let _ = app.emit("charge-to-full-started", ());
    let _ = apply_widget_window(state, app);
    Ok(())
}

fn begin_charge_to_full_session(
    app: &tauri::AppHandle,
    state: &AppState,
    status: &BatteryStatus,
) -> Result<(), String> {
    activate_charge_to_full(app, state, status)
}

fn reapply_charge_to_full(state: &AppState, status: &BatteryStatus) {
    if let Ok(mut backend) = state.backend.lock() {
        // Quiet IOCTL reassert only — never HWND_BROADCAST from background ticks.
        if status.is_plugged && !status.is_charging && status.percent_exact < 99.95 {
            let _ = backend.reassert_charge_to_full(false);
        }
        let _ = backend.reassert_charge_to_full(true);
    }
}

fn tick_charge_monitor(app: &tauri::AppHandle, state: &AppState) {
    let status = {
        let backend = match state.backend.lock() {
            Ok(b) => b,
            Err(_) => return,
        };
        backend.get_status()
    };

    let should_cancel = {
        let monitor = match state.charge_monitor.lock() {
            Ok(m) => m,
            Err(_) => return,
        };
        should_cancel_on_unplug(&monitor) && !status.is_plugged
    };

    if should_cancel {
        if cancel_charge_to_full_session(state).is_ok() {
            let _ = app.emit("charge-to-full-ended", ());
            let _ = apply_widget_window(state, app);
        }
        return;
    }

    // Armed on AC: apply wide-open limits immediately (don't wait for is_charging).
    let should_begin = {
        let monitor = match state.charge_monitor.lock() {
            Ok(m) => m,
            Err(_) => return,
        };
        monitor.armed && !monitor.active && !monitor.complete && status.is_plugged
    };

    if should_begin {
        if begin_charge_to_full_session(app, state, &status).is_ok() {
            // Event emitted inside activate_charge_to_full.
        }
        return;
    }

    // Armed on battery: keep limits released until AC is connected.
    let should_hold_release = {
        let monitor = match state.charge_monitor.lock() {
            Ok(m) => m,
            Err(_) => return,
        };
        monitor.armed && !monitor.active && !monitor.complete
    };
    if should_hold_release {
        reapply_charge_to_full(state, &status);
    }

    // Keep charge-to-full policy applied until the pack is confirmed full.
    let should_reapply = {
        let monitor = match state.charge_monitor.lock() {
            Ok(m) => m,
            Err(_) => return,
        };
        monitor.active && status.is_plugged && !is_battery_full(&status)
    };
    if should_reapply {
        reapply_charge_to_full(state, &status);
    }

    let (should_finish, should_clear, entered_verifying) = {
        let mut monitor = match state.charge_monitor.lock() {
            Ok(m) => m,
            Err(_) => return,
        };

        if monitor.complete {
            let clear = !status.is_plugged || status.percent < 90;
            (false, clear, false)
        } else if !monitor.active {
            (false, false, false)
        } else if status.is_charging {
            monitor.was_charging = true;
            let entered = !monitor.verifying && is_topping_off(&status);
            if is_topping_off(&status) {
                monitor.verifying = true;
            }
            // Hit 100% while still reporting charge — move to complete.
            if status.percent >= 100 {
                (true, false, entered)
            } else {
                (false, false, entered)
            }
        } else if monitor.active && is_battery_full(&status) {
            (true, false, false)
        } else if is_topping_off(&status) && monitor.was_charging {
            let entered = !monitor.verifying;
            monitor.verifying = true;
            (false, false, entered)
        } else {
            (false, false, false)
        }
    };

    if entered_verifying {
        let _ = app.emit("charge-to-full-verifying", ());
    }

    if should_finish {
        if finish_charge_to_full_session(state).is_ok() {
            let _ = app.emit("charge-to-full-complete", ());
            let _ = apply_widget_window(state, app);
        }
        return;
    }

    if should_clear {
        clear_complete_state(state);
        let _ = app.emit("charge-to-full-ended", ());
        let _ = apply_widget_window(state, app);
    }
}

fn start_policy_maintainer(app: tauri::AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(30));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if let Some(state) = app.try_state::<AppState>() {
                let (charge_active, armed, enabled, start, stop) = {
                    let monitor = match state.charge_monitor.lock() {
                        Ok(m) => m,
                        Err(_) => return,
                    };
                    let cfg = match state.config.lock() {
                        Ok(c) => c,
                        Err(_) => return,
                    };
                    (
                        monitor.active,
                        monitor.armed,
                        cfg.thresholds_enabled,
                        cfg.start_threshold,
                        cfg.stop_threshold,
                    )
                };

                if charge_active || armed {
                    let status = {
                        let backend = match state.backend.lock() {
                            Ok(b) => b,
                            Err(_) => return,
                        };
                        backend.get_status()
                    };
                    reapply_charge_to_full(&state, &status);
                } else if enabled {
                    if let Ok(mut backend) = state.backend.lock() {
                        let _ = backend.maintain_policy(false, true, start, stop);
                    }
                }
            }
        }));
        if result.is_err() {
            eprintln!("ThinkCharge: policy maintainer panic recovered; continuing");
        }
    });
}

fn start_charge_monitor(app: tauri::AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(3));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if let Some(state) = app.try_state::<AppState>() {
                tick_charge_monitor(&app, &state);
            }
        }));
        if result.is_err() {
            eprintln!("ThinkCharge: charge monitor panic recovered; continuing");
        }
    });
}

#[tauri::command]
fn get_battery_status(state: State<'_, AppState>) -> Result<BatteryStatus, String> {
    let backend = state.backend.lock().map_err(|e| e.to_string())?;
    Ok(backend.get_status())
}

#[tauri::command]
fn get_threshold_state(state: State<'_, AppState>) -> Result<ThresholdState, String> {
    let backend = state.backend.lock().map_err(|e| e.to_string())?;
    Ok(backend.get_thresholds())
}

#[tauri::command]
fn get_battery_report() -> Result<BatteryReport, String> {
    generate_battery_report()
}

#[tauri::command]
fn get_app_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    merge_runtime_config(&state)
}

#[tauri::command]
fn save_app_config(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    mut config: AppConfig,
) -> Result<(), String> {
    config.charge_to_full_active = false;
    config.charge_to_full_armed = false;
    config.charge_to_full_complete = false;
    config.charge_to_full_verifying = false;
    config.validate()?;
    {
        let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
        *cfg = config;
    }
    let cfg = state.config.lock().map_err(|e| e.to_string())?;
    save_config(&cfg)?;
    drop(cfg);
    apply_thresholds_only(&state)?;
    apply_widget_window(&state, &app)?;
    Ok(())
}

#[tauri::command]
fn charge_to_full(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let status = {
        let backend = state.backend.lock().map_err(|e| e.to_string())?;
        backend.get_status()
    };

    {
        let mut monitor = state.charge_monitor.lock().map_err(|e| e.to_string())?;
        monitor.armed = true;
        monitor.complete = false;
        monitor.verifying = false;
        monitor.was_charging = false;
        if !status.is_plugged {
            monitor.active = false;
        }
    }

    // User-initiated: full registry + IOCTL + broadcast once.
    {
        let mut backend = state.backend.lock().map_err(|e| e.to_string())?;
        if status.is_plugged && !status.is_charging && status.percent_exact < 99.95 {
            let _ = backend.charge_to_full();
        }
        let _ = backend.charge_to_full_top_off();
    }

    if status.is_plugged {
        activate_charge_to_full(&app, &state, &status)?;
    }

    Ok(())
}

#[tauri::command]
fn restore_thresholds(state: State<'_, AppState>) -> Result<(), String> {
    restore_thresholds_internal(&state)
}

#[tauri::command]
fn show_settings(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("settings")
        .ok_or_else(|| "Settings window not found".to_string())?;

    let _ = window.unminimize();
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn toggle_widget(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<bool, String> {
    let mut visible = false;
    if let Some(window) = app.get_webview_window("widget") {
        if window.is_visible().unwrap_or(false) {
            window.hide().map_err(|e| e.to_string())?;
            visible = false;
        } else {
            window.show().map_err(|e| e.to_string())?;
            visible = true;
        }
    }

    {
        let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
        cfg.show_desktop_widget = visible;
        save_config(&cfg)?;
    }

    Ok(visible)
}

#[tauri::command]
fn save_widget_position(
    state: State<'_, AppState>,
    x: i32,
    y: i32,
) -> Result<(), String> {
    let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
    cfg.widget_x = x;
    cfg.widget_y = y;
    save_config(&cfg)
}

#[tauri::command]
fn set_widget_scale(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    scale: f64,
    persist: Option<bool>,
) -> Result<(), String> {
    if scale < 0.85 || scale > 1.75 {
        return Err("Widget scale must be between 0.85 and 1.75".into());
    }
    let save = persist.unwrap_or(true);
    {
        let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
        cfg.widget_scale = scale;
        if save {
            save_config(&cfg)?;
        }
    }
    apply_widget_window(&state, &app)
}

fn build_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let settings = MenuItem::with_id(app, "settings", "Open Settings", true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "Toggle Desktop Widget", true, None::<&str>)?;
    let full = MenuItem::with_id(app, "full", "Charge to Full", true, None::<&str>)?;
    let restore = MenuItem::with_id(
        app,
        "restore",
        "Restore Thresholds",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit ThinkCharge", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &settings,
            &toggle,
            &PredefinedMenuItem::separator(app)?,
            &full,
            &restore,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let icon = app
        .default_window_icon()
        .cloned()
        .expect("missing app icon");

    let tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => {
                let _ = show_settings(app.clone());
            }
            "toggle" => {
                if let Some(state) = app.try_state::<AppState>() {
                    let _ = toggle_widget(app.clone(), state);
                }
            }
            "full" => {
                if let Some(state) = app.try_state::<AppState>() {
                    let _ = charge_to_full(app.clone(), state);
                }
            }
            "restore" => {
                if let Some(state) = app.try_state::<AppState>() {
                    let _ = restore_thresholds(state);
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("widget") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    // Keep the tray alive for the process lifetime (dropping removes the icon).
    app.manage(tray);

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let initial_config = load_config();
    let app_state = AppState {
        config: Mutex::new(initial_config.clone()),
        backend: Mutex::new(create_backend()),
        charge_monitor: Mutex::new(ChargeMonitor::default()),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .setup(move |app| {
            if let Some(state) = app.try_state::<AppState>() {
                // Always start in normal threshold mode — never charge-to-full on launch.
                let _ = apply_thresholds_only(&state);
                let _ = apply_widget_window(&state, &app.handle());
            }

            if let Some(window) = app.get_webview_window("widget") {
                configure_widget_window(&window);
                let _ = window.set_position(tauri::PhysicalPosition::new(
                    initial_config.widget_x,
                    initial_config.widget_y,
                ));
                if !initial_config.show_desktop_widget {
                    let _ = window.hide();
                }
            }

            if let Some(window) = app.get_webview_window("settings") {
                let settings_for_close = window.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = settings_for_close.hide();
                    }
                });
                let _ = window.hide();
            }

            build_tray(&app.handle())?;
            start_charge_monitor(app.handle().clone());
            start_policy_maintainer(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_battery_status,
            get_battery_report,
            get_threshold_state,
            get_app_config,
            save_app_config,
            charge_to_full,
            restore_thresholds,
            show_settings,
            toggle_widget,
            save_widget_position,
            set_widget_scale,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
