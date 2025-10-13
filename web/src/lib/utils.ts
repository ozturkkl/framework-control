export function throttleDebounce<Args extends unknown[], R>(
  func: (...args: Args) => R,
  limit: number,
  leading = true,
  trailing = false
): (...args: Args) => R | void {
  let inThrottle = false;
  let trailingCall: (() => R) | null = null;

  return function (this: unknown, ...args: Args): R | void {
    if (!inThrottle) {
      inThrottle = true;
      setTimeout(() => {
        inThrottle = false;
        if (trailing && trailingCall) {
          trailingCall();
          trailingCall = null;
        }
      }, limit);
      if (leading) {
        return func.apply(this, args);
      } else {
        trailingCall = () => func.apply(this, args);
      }
    } else if (trailing) {
      trailingCall = () => func.apply(this, args);
    }
  };
}

export function deepMerge<T>(
  target: T,
  source: Partial<T>,
  noMergeNulls = false,
  concatArrays = false
) {
  Object.entries(source).forEach(([key, value]) => {
    if (noMergeNulls && (value === null || value === undefined)) return;

    // Handle arrays explicitly (replace by default; concat if enabled)
    if (Array.isArray(value)) {
      const current = target[key as keyof T] as unknown;
      const next = concatArrays
        ? [
            ...(Array.isArray(current) ? (current as unknown[]) : []),
            ...value
          ]
        : value.slice();
      target[key as keyof T] = next as T[keyof T];
      return;
    }

    if (value instanceof Object) {
      target[key as keyof T] = deepMerge(
        target[key as keyof T],
        value as T[keyof T],
        noMergeNulls,
        concatArrays
      );
    } else {
      target[key as keyof T] = value as T[keyof T];
    }
  });
  return target;
}
