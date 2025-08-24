export function computeNaturalCubicSpline(points: [number, number][]): {
  a: number[];
  b: number[];
  c: number[];
  d: number[];
  x: number[];
} {
  const n = points.length;
  const sorted = [...points].sort((a, b) => a[0] - b[0]);

  const x = sorted.map((p) => p[0]);
  const a = sorted.map((p) => p[1]);

  const b = new Array(n - 1);
  const c = new Array(n);
  const d = new Array(n - 1);

  const h = new Array(n - 1);
  for (let i = 0; i < n - 1; i++) h[i] = x[i + 1] - x[i];

  const alpha = new Array(n - 1);
  for (let i = 1; i < n - 1; i++) {
    alpha[i] = (3 / h[i]) * (a[i + 1] - a[i]) - (3 / h[i - 1]) * (a[i] - a[i - 1]);
  }

  const l = new Array(n);
  const mu = new Array(n);
  const z = new Array(n);

  l[0] = 1;
  mu[0] = 0;
  z[0] = 0;
  c[0] = 0;

  for (let i = 1; i < n - 1; i++) {
    l[i] = 2 * (x[i + 1] - x[i - 1]) - h[i - 1] * mu[i - 1];
    mu[i] = h[i] / l[i];
    z[i] = (alpha[i] - h[i - 1] * z[i - 1]) / l[i];
  }

  l[n - 1] = 1;
  z[n - 1] = 0;
  c[n - 1] = 0;

  for (let j = n - 2; j >= 0; j--) {
    c[j] = z[j] - mu[j] * c[j + 1];
    b[j] = (a[j + 1] - a[j]) / h[j] - (h[j] * (c[j + 1] + 2 * c[j])) / 3;
    d[j] = (c[j + 1] - c[j]) / (3 * h[j]);
  }

  return { a: a.slice(0, -1), b, c: c.slice(0, -1), d, x: x.slice(0, -1) };
}

export function evaluateCubicSpline(
  spline: ReturnType<typeof computeNaturalCubicSpline>,
  xVal: number
): number {
  const { a, b, c, d, x } = spline;
  const n = x.length;
  if (n === 0) return xVal;
  if (xVal <= x[0]) return a[0];
  if (xVal >= x[n - 1] + (x[n - 1] - x[n - 2])) {
    const lastX = x[n - 1];
    const dx = xVal - lastX;
    return a[n - 1] + b[n - 1] * dx + c[n - 1] * dx * dx + d[n - 1] * dx * dx * dx;
  }
  let i = 0;
  while (i < n - 1 && x[i + 1] < xVal) i++;
  const dx = xVal - x[i];
  return a[i] + b[i] * dx + c[i] * dx * dx + d[i] * dx * dx * dx;
}

export function cubicSplineInterpolate(points: [number, number][], xVal: number): number {
  if (!points || points.length < 2) return xVal;
  const spline = computeNaturalCubicSpline(points);
  return evaluateCubicSpline(spline, xVal);
}


