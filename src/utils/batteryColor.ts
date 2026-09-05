/** Red (0%) → yellow (50%) → green (100%). */
export function batteryGradientColor(percent: number): string {
  const p = Math.min(100, Math.max(0, percent)) / 100;
  if (p <= 0.5) {
    return lerpColor("#ef4444", "#eab308", p * 2);
  }
  return lerpColor("#eab308", "#22c55e", (p - 0.5) * 2);
}

function lerpColor(from: string, to: string, t: number): string {
  const a = hexToRgb(from);
  const b = hexToRgb(to);
  const mix = (x: number, y: number) => Math.round(x + (y - x) * t);
  return `rgb(${mix(a.r, b.r)}, ${mix(a.g, b.g)}, ${mix(a.b, b.b)})`;
}

function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const n = Number.parseInt(hex.slice(1), 16);
  return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255 };
}
