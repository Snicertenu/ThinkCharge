import { invoke } from "@tauri-apps/api/core";
import type { AppConfig, BatteryStatus, ThresholdState } from "./types";

export function getBatteryStatus(): Promise<BatteryStatus> {
  return invoke("get_battery_status");
}

export function getThresholdState(): Promise<ThresholdState> {
  return invoke("get_threshold_state");
}

export function getAppConfig(): Promise<AppConfig> {
  return invoke("get_app_config");
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

export function setWidgetScale(scale: number): Promise<void> {
  return invoke("set_widget_scale", { scale });
}

export const WIDGET_BASE_WIDTH = 248;
export const WIDGET_BASE_HEIGHT = 198;

export function scaleFromWindowSize(width: number, height: number): number {
  const scaleW = width / WIDGET_BASE_WIDTH;
  const scaleH = height / WIDGET_BASE_HEIGHT;
  return Math.min(scaleW, scaleH);
}

export function windowSizeFromScale(scale: number): { width: number; height: number } {
  return {
    width: Math.round(WIDGET_BASE_WIDTH * scale),
    height: Math.round(WIDGET_BASE_HEIGHT * scale),
  };
}
