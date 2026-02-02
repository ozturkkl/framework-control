export type BrowserInfo = {
  platform: string;
  userAgent: string;
  language: string;
  timezone: string;
  screen: { width: number; height: number; pixelRatio: number };
};

export function getScreenResolution() {
  const dpr = window.devicePixelRatio || 1;
  // Common Windows/Linux scaling factors
  const candidates = [1, 1.25, 1.5, 1.75, 2, 2.5, 3, 4];
  let best = candidates[0];
  let bestDiff = Number.POSITIVE_INFINITY;
  for (const c of candidates) {
    const diff = Math.abs(dpr - c);
    if (diff < bestDiff) {
      bestDiff = diff;
      best = c;
    }
  }
  // Tolerance ~2% for floating errors
  const isLikelyOsScale = bestDiff <= best * 0.02;
  if (!isLikelyOsScale) return null;

  const width = Math.round(window.screen.width * best);
  const height = Math.round(window.screen.height * best);
  // Basic plausibility bounds to avoid bizarre outputs when zoom is involved
  if (width < 800 || height < 600) return null;
  if (width > 16000 || height > 10000) return null;
  return { width, height };
}
