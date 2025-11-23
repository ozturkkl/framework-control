export type BrowserInfo = {
  platform: string;
  userAgent: string;
  language: string;
  timezone: string;
  screen: { width: number; height: number; pixelRatio: number };
};

export function getScreenResolution() {
  const dpr = window.devicePixelRatio || 1;
  // Common Windows/macOS scaling factors
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

export function parseFrameworkVersions(
  text: string | undefined | null
): VersionsSummary {
  const init: VersionsSummary = {
    mainboardType: null,
    mainboardRevision: null,
    uefiVersion: null,
    uefiReleaseDate: null,
    ecBuildVersion: null,
    ecCurrentImage: null,
  };
  if (!text) return init;

  let section = "";
  const lines = text.split(/\r?\n/);
  for (const raw of lines) {
    const line = raw.replace(/\t/g, "    ");
    if (!line.trim()) continue;

    const isSection = !/^\s/.test(line);
    if (isSection) {
      section = line.trim();
      continue;
    }

    const m = line.match(/^\s*([^:]+):\s*(.*)$/);
    if (!m) continue;
    const key = m[1].trim().toLowerCase();
    const value = m[2].trim();

    if (section.toLowerCase().startsWith("mainboard hardware")) {
      if (key === "type") init.mainboardType = value;
      else if (key === "revision") init.mainboardRevision = value;
    } else if (section.toLowerCase().startsWith("uefi bios")) {
      if (key === "version") init.uefiVersion = value;
      else if (key === "release date") init.uefiReleaseDate = value;
    } else if (section.toLowerCase().startsWith("ec firmware")) {
      if (key === "build version") init.ecBuildVersion = value;
      else if (key === "current image") init.ecCurrentImage = value;
    }
  }
  return init;
}
