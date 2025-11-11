// Distinct, readable palette for sensor series
export const SERIES_PALETTE = [
  "#22c55e", // green
  "#3b82f6", // blue
  "#ef4444", // red
  "#a855f7", // purple
  "#f59e0b", // amber
  "#10b981", // emerald
  "#f97316", // orange
  "#06b6d4", // cyan
  "#eab308", // yellow
  "#8b5cf6", // violet
  "#ec4899", // pink
  "#14b8a6", // teal
];

export function hashColor(name: string): string {
  let h = 0 >>> 0;
  for (let i = 0; i < name.length; i++) {
    h = (h * 31 + name.charCodeAt(i)) >>> 0;
  }
  return SERIES_PALETTE[h % SERIES_PALETTE.length];
}


