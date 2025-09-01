export function parseSemver(versionText: string): number[] {
  return versionText
    .trim()
    .split(".")
    .map((segment) => parseInt(segment, 10))
    .map((value) => (isNaN(value) ? 0 : value));
}

export function gtSemver(a: string, b: string): boolean {
  const aParts = parseSemver(a);
  const bParts = parseSemver(b);
  const length = Math.max(aParts.length, bParts.length);
  for (let i = 0; i < length; i++) {
    const av = aParts[i] ?? 0;
    const bv = bParts[i] ?? 0;
    if (av > bv) return true;
    if (av < bv) return false;
  }
  return false;
}


