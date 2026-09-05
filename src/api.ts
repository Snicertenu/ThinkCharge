import { invoke } from "@tauri-apps/api/core";
import type { AppConfig, BatteryReport, BatteryStatus, ThresholdState } from "./types";

export function getBatteryStatus(): Promise<BatteryStatus> {
  return invoke("get_battery_status");
}

export function getThresholdState(): Promise<ThresholdState> {
  return invoke("get_threshold_state");
}

export function getAppConfig(): Promise<AppConfig> {
  return invoke("get_app_config");
}

export function getBatteryReport(): Promise<BatteryReport> {
  return invoke("get_battery_report");
}

export function saveAppConfig(config: AppConfig): Promise<void> {
  return invoke("save_app_config", { config });
}

export function chargeToFull(): Promise<void> {
  return invoke("charge_to_full");
}

export function restoreThresholds(): Promise<void> {
  return invoke("restore_thresholds");
}

export function showSettings(): Promise<void> {
  return invoke("show_settings");
}

export function toggleWidget(): Promise<boolean> {
  return invoke("toggle_widget");
}

export function saveWidgetPosition(x: number, y: number): Promise<void> {
  return invoke("save_widget_position", { x, y });
}

export function setWidgetScale(scale: number, persist = true): Promise<void> {
  return invoke("set_widget_scale", { scale, persist });
}

export const WIDGET_BASE_WIDTH = 248;
export const WIDGET_BASE_HEIGHT = 168;
export const WIDGET_CHARGE_BAR_EXTRA = 28;

export function effectiveWidgetBaseHeight(showChargeBar: boolean): number {
  return WIDGET_BASE_HEIGHT + (showChargeBar ? WIDGET_CHARGE_BAR_EXTRA : 0);
}

export function scaleFromWindowSize(
  width: number,
  height: number,
  showChargeBar = false,
): number {
  const baseH = effectiveWidgetBaseHeight(showChargeBar);
  const scaleW = width / WIDGET_BASE_WIDTH;
  const scaleH = height / baseH;
  return Math.min(scaleW, scaleH);
}

export function windowSizeFromScale(scale: number, showChargeBar = false): {
  width: number;
  height: number;
} {
  const baseH = effectiveWidgetBaseHeight(showChargeBar);
  return {
    width: Math.round(WIDGET_BASE_WIDTH * scale),
    height: Math.round(baseH * scale),
  };
}
