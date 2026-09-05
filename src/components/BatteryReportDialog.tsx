import type { BatteryReport } from "../types";

function fmt(value: string | number | null | undefined, suffix = ""): string {
  if (value === null || value === undefined || value === "") return "—";
  return `${value}${suffix}`;
}

function healthClass(rating: string): string {
  const r = rating.toLowerCase();
  if (r === "good") return "report-health--good";
  if (r === "fair") return "report-health--fair";
  if (r === "poor") return "report-health--poor";
  return "report-health--unknown";
}

interface BatteryReportDialogProps {
  report: BatteryReport;
  onClose: () => void;
}

export function BatteryReportDialog({ report, onClose }: BatteryReportDialogProps) {
  const generated = new Date(report.generatedAtUnix * 1000).toLocaleString();

  return (
    <div className="report-dialog-backdrop" role="presentation" onClick={onClose}>
      <div
        className="report-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="battery-report-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="report-dialog-header">
          <h2 id="battery-report-title">Battery Report</h2>
          <button type="button" className="report-dialog-close" onClick={onClose} aria-label="Close">
            ×
          </button>
        </div>

        <p className="report-dialog-meta">
          Generated {generated} · {report.platform}
        </p>

        <div className={`report-health ${healthClass(report.healthRating)}`}>
          <span className="report-health__rating">{report.healthRating}</span>
          <span className="report-health__soc">
            {report.stateOfHealthPercent != null
              ? `${Math.round(report.stateOfHealthPercent)}% health`
              : "Health unknown"}
          </span>
          <p className="report-health__summary">{report.healthSummary}</p>
        </div>

        <dl className="report-grid">
          <div>
            <dt>Charge level</dt>
            <dd>{Math.round(report.stateOfChargePercent)}%</dd>
          </div>
          <div>
            <dt>State</dt>
            <dd>{report.state}</dd>
          </div>
          <div>
            <dt>Power</dt>
            <dd>
              {report.isPlugged ? "Plugged in" : "On battery"}
              {report.isCharging ? " · Charging" : " · Not charging"}
            </dd>
          </div>
          <div>
            <dt>Vendor</dt>
            <dd>{fmt(report.vendor)}</dd>
          </div>
          <div>
            <dt>Model / part</dt>
            <dd>{fmt(report.model)}</dd>
          </div>
          <div>
            <dt>Serial</dt>
            <dd>{fmt(report.serialNumber)}</dd>
          </div>
          <div>
            <dt>Technology</dt>
            <dd>{report.technology}</dd>
          </div>
          <div>
            <dt>Full capacity</dt>
            <dd>{report.energyFullWh != null ? `${report.energyFullWh.toFixed(1)} Wh` : "—"}</dd>
          </div>
          <div>
            <dt>Design capacity</dt>
            <dd>{report.energyDesignWh != null ? `${report.energyDesignWh.toFixed(1)} Wh` : "—"}</dd>
          </div>
          <div>
            <dt>Voltage</dt>
            <dd>{report.voltageV != null ? `${report.voltageV.toFixed(2)} V` : "—"}</dd>
          </div>
          <div>
            <dt>Temperature</dt>
            <dd>{report.temperatureC != null ? `${report.temperatureC.toFixed(1)} °C` : "—"}</dd>
          </div>
          <div>
            <dt>Cycle count</dt>
            <dd>{fmt(report.cycleCount)}</dd>
          </div>
          <div>
            <dt>Time to empty</dt>
            <dd>{report.timeToEmptyMin != null ? `${report.timeToEmptyMin} min` : "—"}</dd>
          </div>
          <div>
            <dt>Time to full</dt>
            <dd>{report.timeToFullMin != null ? `${report.timeToFullMin} min` : "—"}</dd>
          </div>
        </dl>

        {report.replacementHint && (
          <p className="report-hint">{report.replacementHint}</p>
        )}

        <div className="report-dialog-actions">
          <button type="button" className="settings-primary" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
