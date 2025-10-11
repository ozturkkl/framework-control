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
  noMergeNulls = false
) {
  Object.entries(source).forEach(([key, value]) => {
    if (value instanceof Object) {
      target[key as keyof T] = deepMerge(
        target[key as keyof T],
        value as T[keyof T],
        noMergeNulls
      );
    } else {
      if (noMergeNulls && (value === null || value === undefined)) return;
      target[key as keyof T] = value as T[keyof T];
    }
  });
  return target;
}
