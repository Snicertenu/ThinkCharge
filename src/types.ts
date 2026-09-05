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
  useStatusIcons: boolean;
  matchOsTheme: boolean;
}

export interface BatteryReport {
  generatedAtUnix: number;
  platform: string;
  vendor?: string | null;
  model?: string | null;
  serialNumber?: string | null;
  technology: string;
  state: string;
  stateOfChargePercent: number;
  stateOfHealthPercent?: number | null;
  healthRating: string;
  healthSummary: string;
  energyFullWh?: number | null;
  energyDesignWh?: number | null;
  voltageV?: number | null;
  temperatureC?: number | null;
  cycleCount?: number | null;
  isCharging: boolean;
  isPlugged: boolean;
  timeToEmptyMin?: number | null;
  timeToFullMin?: number | null;
  replacementHint?: string | null;
}

export type SystemTheme = "light" | "dark";
