export interface BatteryStatus {
  percent: number;
  percentExact: number;
  isCharging: boolean;
  isPlugged: boolean;
  timeRemainingSec?: number | null;
}

export interface ThresholdState {
  supported: boolean;
  enabled: boolean;
  start?: number | null;
  stop?: number | null;
  backend: string;
  message: string;
}

export interface AppConfig {
  startThreshold: number;
  stopThreshold: number;
  thresholdsEnabled: boolean;
  showDesktopWidget: boolean;
  widgetX: number;
  widgetY: number;
  widgetOpacity: number;
  widgetScale: number;
  chargeToFullActive: boolean;
  chargeToFullArmed: boolean;
  chargeToFullComplete: boolean;
  chargeToFullVerifying: boolean;
  showBatteryPercent: boolean;
  showPowerSource: boolean;
  showChargingStatus: boolean;
  matchOsTheme: boolean;
}

export type SystemTheme = "light" | "dark";
