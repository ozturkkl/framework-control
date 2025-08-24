export function parseThermalOutput(stdout: string | undefined | null): {
  temps: Record<string, number>;
  rpms: number[];
} {
  const temps: Record<string, number> = {};
  const rpms: number[] = [];
  if (!stdout) return { temps, rpms };
  const lines = stdout.split(/\r?\n/);
  for (const line of lines) {
    const l = line.trim();
    const mTemp = l.match(/^([A-Za-z0-9_ ]+):\s*(-?\d+)\s*C\b/i);
    if (mTemp) {
      const key = mTemp[1].trim();
      const val = parseInt(mTemp[2], 10);
      if (!Number.isNaN(val)) temps[key] = val;
      continue;
    }
    const mRpm = l.match(/Fan\s+Speed:\s*(\d+)\s*RPM/i);
    if (mRpm) {
      const rpm = parseInt(mRpm[1], 10);
      if (!Number.isNaN(rpm) && rpm > 0) rpms.push(rpm);
    }
  }
  return { temps, rpms };
}

export function pickTempForSensor(
  temps: Record<string, number>,
  which: string
): number | null {
  const norm = (k: string) => k.replace(/\s+/g, "").toUpperCase();
  const map = new Map<string, number>();
  for (const k of Object.keys(temps)) map.set(norm(k), temps[k]);

  const candidates =
    which === "APU"
      ? ["APU", "F75303_APU", "F75303LOCAL", "LOCAL", "CPU", "F75303_CPU", "PECI"]
      : ["F75303_CPU", "CPU", "PECI", "APU", "F75303_APU"];
  for (const c of candidates) {
    if (map.has(c)) return map.get(c)!;
  }
  const first = Object.values(temps)[0];
  return typeof first === "number" ? first : null;
}


