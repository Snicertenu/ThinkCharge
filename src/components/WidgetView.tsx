import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  chargeToFull,
  getAppConfig,
  getBatteryStatus,
  saveWidgetPosition,
  scaleFromWindowSize,
  setWidgetScale,
  showSettings,
} from "../api";
import { useSystemTheme } from "../hooks/useSystemTheme";
import { ChargeBar } from "./ChargeBar";
import { WidgetStatus } from "./WidgetStatus";
import type { AppConfig, BatteryStatus } from "../types";

function modeLabel(config: AppConfig | null): string {
  if (!config) return "Loading…";
  if (config.chargeToFullComplete) return "Battery at 100%";
  if (config.chargeToFullVerifying) return "Verifying 100% — topping off";
  if (config.chargeToFullActive) return "Charging to 100%";
  if (config.chargeToFullArmed) return "Will charge to 100% when plugged in";
  if (config.thresholdsEnabled) {
    return `${config.startThreshold}% → ${config.stopThreshold}%`;
  }
  return "Thresholds off";
}

function chargeButtonLabel(config: AppConfig | null): string {
  if (!config) return "Charge to Full";
  if (config.chargeToFullComplete) return "Battery at 100%";
  if (config.chargeToFullVerifying) return "Verifying 100%…";
  if (config.chargeToFullActive) return "Charging to Full…";
  if (config.chargeToFullArmed) return "Waiting for AC power…";
  return "Charge to Full";
}

function showChargeBar(config: AppConfig | null): boolean {
  if (!config) return false;
  return (
    config.chargeToFullActive ||
    config.chargeToFullVerifying ||
    config.chargeToFullComplete
  );
}

function chargeSessionActive(config: AppConfig | null): boolean {
  if (!config) return false;
  return (
    config.chargeToFullActive ||
    config.chargeToFullVerifying ||
    config.chargeToFullArmed ||
    config.chargeToFullComplete
  );
}

function clampScale(scale: number): number {
  return Math.min(1.75, Math.max(0.85, Number(scale.toFixed(2))));
}

function GearIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="1.8">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M12 15.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7Z"
      />
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M19.4 13a7.8 7.8 0 0 0 .1-2l2-1.2-2-3.4-2.3.7a7.6 7.6 0 0 0-1.7-1L15 3.5h-6L8.5 6a7.6 7.6 0 0 0-1.7 1L4.5 6.3l-2 3.4 2 1.2a7.8 7.8 0 0 0 0 2l-2 1.2 2 3.4 2.3-.7a7.6 7.6 0 0 0 1.7 1L9 20.5h6l.5-2.5a7.6 7.6 0 0 0 1.7-1l2.3.7 2-3.4-2-1.2Z"
      />
    </svg>
  );
}

export function WidgetView() {
  const [status, setStatus] = useState<BatteryStatus | null>(null);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [previewScale, setPreviewScale] = useState<number | null>(null);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const resizeRef = useRef<{ startY: number; startScale: number } | null>(null);
  const draggingRef = useRef(false);
  const rafRef = useRef<number | null>(null);
  const pendingScaleRef = useRef<number | null>(null);
  const lastWindowSyncRef = useRef(0);
  const configRef = useRef(config);
  const window = getCurrentWindow();

  configRef.current = config;

  useSystemTheme(config?.matchOsTheme ?? true, false);

  const refresh = useCallback(async () => {
    const [battery, cfg] = await Promise.all([getBatteryStatus(), getAppConfig()]);
    setStatus(battery);
    setConfig(cfg);
  }, []);

  const refreshInFlight = useRef(false);
  const safeRefresh = useCallback(async () => {
    if (refreshInFlight.current) return;
    refreshInFlight.current = true;
    try {
      await refresh();
    } catch {
      // Keep the last good UI state if a poll fails (e.g. transient IPC stall).
    } finally {
      refreshInFlight.current = false;
    }
  }, [refresh]);

  useEffect(() => {
    void safeRefresh();
    const intervalMs = chargeSessionActive(config) ? 2000 : 4000;
    const timer = setInterval(() => void safeRefresh(), intervalMs);
    return () => clearInterval(timer);
  }, [safeRefresh, config?.chargeToFullActive, config?.chargeToFullArmed, config?.chargeToFullComplete, config?.chargeToFullVerifying]);

  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    void listen("charge-to-full-ended", () => void safeRefresh()).then((fn) =>
      unlisteners.push(fn),
    );
    void listen("charge-to-full-started", () => void safeRefresh()).then((fn) =>
      unlisteners.push(fn),
    );
    void listen("charge-to-full-complete", () => void safeRefresh()).then((fn) =>
      unlisteners.push(fn),
    );
    return () => {
      for (const fn of unlisteners) fn();
    };
  }, [safeRefresh]);

  // Sync scale when the user resizes via window edges (not the drag handle).
  useEffect(() => {
    let resizeTimer: ReturnType<typeof setTimeout> | undefined;
    let unlisten: (() => void) | undefined;

    const onResize = () => {
      if (draggingRef.current) return;
      if (resizeTimer) clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => {
        void window.innerSize().then(async (size) => {
          if (draggingRef.current) return;
          const next = clampScale(
            scaleFromWindowSize(
              size.width,
              size.height,
              showChargeBar(configRef.current),
            ),
          );
          setPreviewScale(null);
          setConfig((prev) => (prev ? { ...prev, widgetScale: next } : prev));
          await setWidgetScale(next, true);
        });
      }, 200);
    };

    void window.onResized(onResize).then((fn) => {
      unlisten = fn;
    });
    return () => {
      if (resizeTimer) clearTimeout(resizeTimer);
      unlisten?.();
    };
  }, [window]);

  const commitScale = useCallback(async (scale: number, persist: boolean) => {
    const next = clampScale(scale);
    await setWidgetScale(next, persist);
    setConfig((prev) => (prev ? { ...prev, widgetScale: next } : prev));
  }, []);

  const onDragEnd = async () => {
    const pos = await window.outerPosition();
    await saveWidgetPosition(pos.x, pos.y);
  };

  const onConfirmChargeToFull = async () => {
    setConfirmOpen(false);
    await chargeToFull();
    await refresh();
  };

  const onResizeStart = (event: React.MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
    if (!config) return;
    draggingRef.current = true;
    resizeRef.current = { startY: event.clientY, startScale: config.widgetScale };
  };

  useEffect(() => {
    const schedulePreview = (scale: number) => {
      pendingScaleRef.current = scale;
      if (rafRef.current !== null) return;
      rafRef.current = requestAnimationFrame(() => {
        rafRef.current = null;
        if (pendingScaleRef.current !== null) {
          setPreviewScale(pendingScaleRef.current);
        }
      });
    };

    const onMove = (event: MouseEvent) => {
      if (!resizeRef.current) return;
      const delta = event.clientY - resizeRef.current.startY;
      const next = clampScale(resizeRef.current.startScale + delta / 180);
      schedulePreview(next);

      const now = performance.now();
      if (now - lastWindowSyncRef.current > 80) {
        lastWindowSyncRef.current = now;
        void setWidgetScale(next, false);
      }
    };

    const onUp = () => {
      if (!resizeRef.current) return;
      const finalScale = pendingScaleRef.current ?? previewScale ?? config?.widgetScale ?? 1.05;
      resizeRef.current = null;
      draggingRef.current = false;
      pendingScaleRef.current = null;
      setPreviewScale(null);
      void commitScale(finalScale, true);
    };

    globalThis.addEventListener("mousemove", onMove);
    globalThis.addEventListener("mouseup", onUp);
    return () => {
      globalThis.removeEventListener("mousemove", onMove);
      globalThis.removeEventListener("mouseup", onUp);
      if (rafRef.current !== null) {
        cancelAnimationFrame(rafRef.current);
      }
    };
  }, [commitScale, config?.widgetScale, previewScale]);

  const scale = previewScale ?? config?.widgetScale ?? 1.05;

  return (
    <div
      className="widget-shell"
      style={{ ["--widget-scale" as string]: String(scale) }}
    >
      <div className="widget-panel" onMouseUp={() => void onDragEnd()}>
        <div className="widget-header" data-tauri-drag-region>
          <span className="widget-title">ThinkCharge</span>
          <div className="widget-header-actions">
            <button
              type="button"
              className="widget-icon-btn"
              onClick={() => void showSettings()}
              aria-label="Open settings"
              title="Settings"
            >
              <GearIcon />
            </button>
            <button
              type="button"
              className="widget-icon-btn"
              onClick={() => void window.hide()}
              aria-label="Hide widget"
              title="Hide"
            >
              ×
            </button>
          </div>
        </div>

        {status && config ? (
          <WidgetStatus status={status} config={config} />
        ) : (
          <p className="widget-status-row">…</p>
        )}
        <p className="widget-mode">{modeLabel(config)}</p>

        <ChargeBar
          percent={status?.percent ?? 0}
          percentExact={status?.percentExact}
          active={showChargeBar(config)}
          complete={config?.chargeToFullComplete ?? false}
          verifying={config?.chargeToFullVerifying ?? false}
        />

        <button
          type="button"
          className="widget-primary-btn"
          disabled={chargeSessionActive(config)}
          onClick={() => setConfirmOpen(true)}
        >
          {chargeButtonLabel(config)}
        </button>

        <button
          type="button"
          className="widget-resize-handle"
          aria-label="Resize widget"
          onMouseDown={onResizeStart}
        />
      </div>

      {confirmOpen && (
        <div className="widget-dialog-backdrop" role="presentation">
          <div className="widget-dialog" role="dialog" aria-modal="true" aria-labelledby="charge-dialog-title">
            <h2 id="charge-dialog-title" className="widget-dialog-title">
              Charge to Full
            </h2>
            <p className="widget-dialog-body">
              This is a one-time override. ThinkCharge will disable your charge
              limits right away, charge to 100% when you connect AC, then
              automatically restore your saved thresholds once the battery is
              confirmed full.
            </p>
            <div className="widget-dialog-actions">
              <button type="button" className="widget-secondary-btn" onClick={() => setConfirmOpen(false)}>
                Cancel
              </button>
              <button type="button" className="widget-primary-btn" onClick={() => void onConfirmChargeToFull()}>
                Start Charge to Full
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
