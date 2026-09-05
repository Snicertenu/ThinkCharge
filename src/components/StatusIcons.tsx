interface IconProps {
  className?: string;
}

export function BatteryIcon({ className = "status-icon" }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" className={className} fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden>
      <rect x="3" y="7" width="16" height="10" rx="2" />
      <path d="M21 10v4" strokeLinecap="round" />
      <path d="M7 11h6" strokeLinecap="round" />
    </svg>
  );
}

export function PlugIcon({ className = "status-icon" }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" className={className} fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden>
      <path d="M7 7v4a5 5 0 0 0 10 0V7" strokeLinecap="round" />
      <path d="M9 3v4M15 3v4" strokeLinecap="round" />
      <path d="M12 16v5" strokeLinecap="round" />
    </svg>
  );
}

export function LightningIcon({ className = "status-icon" }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" className={className} fill="currentColor" aria-hidden>
      <path d="M13 2 4 14h7l-1 8 9-12h-7l1-8Z" />
    </svg>
  );
}

export function LightningOffIcon({ className = "status-icon" }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" className={className} fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden>
      <path d="M13 2 4 14h7l-1 8 9-12h-7l1-8Z" fill="currentColor" stroke="none" opacity="0.45" />
      <path d="m4 4 16 16" strokeLinecap="round" />
    </svg>
  );
}
