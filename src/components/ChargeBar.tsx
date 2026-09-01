interface ChargeBarProps {
  percent: number;
  percentExact?: number;
  active: boolean;
  complete?: boolean;
  verifying?: boolean;
}

export function ChargeBar({
  percent,
  percentExact,
  active,
  complete = false,
  verifying = false,
}: ChargeBarProps) {
  if (!active) return null;

  const exact = percentExact ?? percent;
  const barWidth = Math.min(100, Math.max(0, exact));
  const displayPercent = percent;
  const isFull = complete;

  let modeClass = "charge-bar--charging";
  if (isFull) modeClass = "charge-bar--full";
  else if (verifying) modeClass = "charge-bar--verifying";

  let centerLabel = `${displayPercent}%`;
  if (isFull) centerLabel = "Battery at 100%";
  else if (verifying) centerLabel = `Verifying 100% (${displayPercent}%)`;

  return (
    <div
      className={`charge-bar ${modeClass}`}
      role="progressbar"
      aria-valuenow={isFull ? 100 : displayPercent}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-label={
        isFull
          ? "Battery at 100%"
          : verifying
            ? `Verifying battery at ${displayPercent} percent`
            : `Battery charging ${displayPercent} percent`
      }
    >
      <div className="charge-bar__track">
        <div
          className={`charge-bar__fill ${isFull ? "charge-bar__fill--full" : verifying ? "charge-bar__fill--verify" : "charge-bar__fill--wave"}`}
          style={{ width: `${isFull ? 100 : barWidth}%` }}
        />
        {!isFull && (
          <div
            className="charge-bar__wave"
            style={{ width: `${barWidth}%` }}
            aria-hidden
          />
        )}
      </div>
      <div className="charge-bar__labels">
        <span>0%</span>
        <span>{centerLabel}</span>
        <span>100%</span>
      </div>
    </div>
  );
}
