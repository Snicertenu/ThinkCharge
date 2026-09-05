import type { ReactNode } from "react";
import { batteryGradientColor } from "../utils/batteryColor";
import type { AppConfig, BatteryStatus } from "../types";
import { BatteryIcon, LightningIcon, LightningOffIcon, PlugIcon } from "./StatusIcons";

interface WidgetStatusProps {
  status: BatteryStatus;
  config: AppConfig;
}

function IconWrap({ label, children }: { label: string; children: ReactNode }) {
  return (
    <span className="widget-status__icon" title={label} aria-label={label}>
      {children}
    </span>
  );
}

function buildTextParts(status: BatteryStatus, config: AppConfig): string[] {
  const parts: string[] = [];
  if (config.showPowerSource) {
    parts.push(status.isPlugged ? "AC power" : "On battery");
  }
  if (config.showChargingStatus) {
    parts.push(status.isCharging ? "Charging" : "Not charging");
  }
  return parts;
}

export function WidgetStatus({ status, config }: WidgetStatusProps) {
  const percentColor = batteryGradientColor(status.percentExact ?? status.percent);
  const textParts = buildTextParts(status, config);
  const useIcons = config.useStatusIcons;

  return (
    <div className="widget-status-row">
      {config.showBatteryPercent && (
        <span
          className="widget-status__percent"
          style={{ color: percentColor }}
          title={`${status.percentExact?.toFixed(1) ?? status.percent}%`}
        >
          {status.percent}%
        </span>
      )}

      {(config.showPowerSource || config.showChargingStatus) && (
        <span className="widget-status__indicators">
          {useIcons ? (
            <>
              {config.showPowerSource && (
                <IconWrap label={status.isPlugged ? "AC power" : "On battery"}>
                  {status.isPlugged ? <PlugIcon /> : <BatteryIcon />}
                </IconWrap>
              )}
              {config.showChargingStatus && (
                <IconWrap label={status.isCharging ? "Charging" : "Not charging"}>
                  {status.isCharging ? <LightningIcon /> : <LightningOffIcon />}
                </IconWrap>
              )}
            </>
          ) : (
            textParts.length > 0 && (
              <span className="widget-status__text">{textParts.join(" · ")}</span>
            )
          )}
        </span>
      )}
    </div>
  );
}
