export type BrowserInfo = {
  platform: string;
  userAgent: string;
  language: string;
  timezone: string;
  screen: { width: number; height: number; pixelRatio: number };
};

export function getScreenResolution(): { width: number; height: number } {
  return {
    width: window.screen.width,
    height: window.screen.height,
  };
}

export type VersionsSummary = {
  mainboardType: string | null;
  mainboardRevision: string | null;
  uefiVersion: string | null;
  uefiReleaseDate: string | null;
  ecBuildVersion: string | null;
  ecCurrentImage: string | null;
};

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
