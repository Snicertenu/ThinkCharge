import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  getAppConfig,
  getBatteryReport,
  getThresholdState,
  saveAppConfig,
  setWidgetScale,
} from "../api";
import { useSystemTheme } from "../hooks/useSystemTheme";
import { BatteryReportDialog } from "./BatteryReportDialog";
import type { AppConfig, BatteryReport, ThresholdState } from "../types";

export function SettingsView() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [thresholds, setThresholds] = useState<ThresholdState | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [report, setReport] = useState<BatteryReport | null>(null);
  const [reportLoading, setReportLoading] = useState(false);
  const scaleTimerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  useSystemTheme(config?.matchOsTheme ?? true);

  useEffect(() => {
    document.documentElement.dataset.window = "settings";
  }, []);

  useEffect(() => {
    void Promise.all([getAppConfig(), getThresholdState()]).then(([cfg, state]) => {
      setConfig(cfg);
      setThresholds(state);
    });
  }, []);

  if (!config) {
    return <div className="settings-shell">Loading settings…</div>;
  }

  const update = (patch: Partial<AppConfig>) => {
    setConfig({ ...config, ...patch });
    setSaved(false);
  };

  const onSave = async () => {
    setError(null);
    try {
      const next = { ...config, chargeToFullActive: false };
      await saveAppConfig(next);
      setConfig(next);
      setSaved(true);
    } catch (e) {
      setError(String(e));
    }
  };

  const onScalePreview = (scale: number) => {
    const next = Math.min(1.75, Math.max(0.85, scale));
    update({ widgetScale: next });
    if (scaleTimerRef.current) clearTimeout(scaleTimerRef.current);
    scaleTimerRef.current = setTimeout(() => {
      void setWidgetScale(next, false).catch((e) => setError(String(e)));
    }, 16);
  };

  const onScaleCommit = () => {
    if (!config) return;
    if (scaleTimerRef.current) clearTimeout(scaleTimerRef.current);
    void setWidgetScale(config.widgetScale, true).catch((e) => setError(String(e)));
  };

  const onGenerateReport = async () => {
    setError(null);
    setReportLoading(true);
    try {
      const data = await getBatteryReport();
      setReport(data);
    } catch (e) {
      setError(String(e));
    } finally {
      setReportLoading(false);
    }
  };

  return (
    <div className="settings-shell">
      <h1 className="settings-title">Battery Thresholds</h1>
      <p className="settings-subtitle">{thresholds?.message || thresholds?.backend}</p>

      <section className="settings-section">
        <h2 className="settings-section-title">Thresholds</h2>

        <label className="settings-field">
          <span>Start charging below (%)</span>
          <input
            type="range"
            min={0}
            max={95}
            value={config.startThreshold}
            onChange={(e) => update({ startThreshold: Number(e.target.value) })}
          />
          <strong>{config.startThreshold}%</strong>
        </label>

        <label className="settings-field">
          <span>Stop charging at (%)</span>
          <input
            type="range"
            min={5}
            max={100}
            value={config.stopThreshold}
            onChange={(e) => update({ stopThreshold: Number(e.target.value) })}
          />
          <strong>{config.stopThreshold}%</strong>
        </label>

        <label className="settings-check">
          <input
            type="checkbox"
            checked={config.thresholdsEnabled}
            onChange={(e) => update({ thresholdsEnabled: e.target.checked })}
          />
          Enable charge thresholds (AC power passthrough)
        </label>
      </section>

      <section className="settings-section">
        <h2 className="settings-section-title">Display Information</h2>

        <label className="settings-check">
          <input
            type="checkbox"
            checked={config.showBatteryPercent}
            onChange={(e) => update({ showBatteryPercent: e.target.checked })}
          />
          Show battery percentage
        </label>

        <label className="settings-check">
          <input
            type="checkbox"
            checked={config.showPowerSource}
            onChange={(e) => update({ showPowerSource: e.target.checked })}
          />
          Show power source (AC / On battery)
        </label>

        <label className="settings-check">
          <input
            type="checkbox"
            checked={config.showChargingStatus}
            onChange={(e) => update({ showChargingStatus: e.target.checked })}
          />
          Show charging status
        </label>

        <label className="settings-check">
          <input
            type="checkbox"
            checked={config.useStatusIcons}
            onChange={(e) => update({ useStatusIcons: e.target.checked })}
          />
          Use icons for power and charging (off = text labels)
        </label>

        <label className="settings-check">
          <input
            type="checkbox"
            checked={config.matchOsTheme}
            onChange={(e) => update({ matchOsTheme: e.target.checked })}
          />
          Match operating system theme and window effects
        </label>
      </section>

      <section className="settings-section">
        <h2 className="settings-section-title">Battery Report</h2>
        <p className="settings-hint">
          View hardware details, health rating, and replacement part info from your system.
        </p>
        <button
          type="button"
          className="settings-secondary"
          disabled={reportLoading}
          onClick={() => void onGenerateReport()}
        >
          {reportLoading ? "Generating…" : "Generate Battery Report"}
        </button>
      </section>

      <section className="settings-section">
        <h2 className="settings-section-title">Widget</h2>

        <label className="settings-field">
          <span>Widget size</span>
          <input
            type="range"
            min={0.85}
            max={1.75}
            step={0.05}
            value={config.widgetScale}
            onChange={(e) => onScalePreview(Number(e.target.value))}
            onMouseUp={onScaleCommit}
            onTouchEnd={onScaleCommit}
          />
          <strong>{Math.round(config.widgetScale * 100)}%</strong>
        </label>

        <label className="settings-check">
          <input
            type="checkbox"
            checked={config.showDesktopWidget}
            onChange={(e) => update({ showDesktopWidget: e.target.checked })}
          />
          Show desktop widget on startup
        </label>

        <p className="settings-hint">
          Drag the corner handle on the widget to resize, or use the slider above.
        </p>
      </section>

      {error && <p className="settings-error">{error}</p>}
      {saved && <p className="settings-success">Settings saved and applied.</p>}

      <p className="settings-hint">
        Keep stop at least 4% above start. Charge to 100% monthly for battery health.
      </p>

      <div className="settings-actions">
        <button type="button" className="settings-primary" onClick={() => void onSave()}>
          Save & Apply
        </button>
        <button type="button" className="settings-secondary" onClick={() => void getCurrentWindow().hide()}>
          Close
        </button>
      </div>

      {report && <BatteryReportDialog report={report} onClose={() => setReport(null)} />}
    </div>
  );
}
